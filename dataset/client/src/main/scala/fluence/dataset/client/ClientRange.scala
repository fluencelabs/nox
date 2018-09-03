/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

package fluence.dataset.client

import cats.effect.{Effect, IO}
import cats.syntax.applicativeError._
import cats.syntax.functor._
import com.google.protobuf.ByteString
import fluence.btree.core.{Hash, Key}
import fluence.btree.protocol.BTreeRpc
import fluence.dataset.protobuf._
import fluence.dataset.protocol.{ClientError, ServerError}
import monix.execution.Scheduler
import monix.reactive.Observable
import monix.reactive.subjects.PublishSubject

import scala.collection.Searching
import scala.concurrent.Future
import scala.language.higherKinds

class ClientRange[F[_]: Effect](datasetId: Array[Byte], version: Long, rangeCallbacks: BTreeRpc.SearchCallback[F])
    extends slogging.LazyLogging {

  private def handleClientErr(err: Throwable): ErrorFromClient[RangeCallbackReply] = {
    ErrorFromClient(RangeCallbackReply(RangeCallbackReply.Reply.ClientError(Error(err.getMessage))))
  }

  private def handleAsks(source: Observable[RangeCallback.Callback])(
    implicit sch: Scheduler
  ): Observable[Flow[RangeCallbackReply]] =
    source
      .mapEval[F, Flow[RangeCallbackReply]] {
        handleContinuation orElse handleResult
      }

  private def handleContinuation(
    implicit scheduler: Scheduler
  ): PartialFunction[RangeCallback.Callback, F[Flow[RangeCallbackReply]]] = {
    case ask if ask.isNextChildIndex ⇒
      val Some(nci) = ask.nextChildIndex

      rangeCallbacks
        .nextChildIndex(
          nci.keys.map(k ⇒ Key(k.toByteArray)).toArray,
          nci.childsChecksums.map(c ⇒ Hash(c.toByteArray)).toArray
        )
        .attempt
        .map {
          case Left(err) ⇒
            handleClientErr(err)
          case Right(idx) ⇒
            Continuation(RangeCallbackReply(RangeCallbackReply.Reply.NextChildIndex(ReplyNextChildIndex(idx))))
        }

    case ask if ask.isSubmitLeaf ⇒
      val Some(sl) = ask.submitLeaf

      rangeCallbacks
        .submitLeaf(
          sl.keys.map(k ⇒ Key(k.toByteArray)).toArray,
          sl.valuesChecksums.map(c ⇒ Hash(c.toByteArray)).toArray
        )
        .attempt
        .map {
          case Left(err) ⇒
            handleClientErr(err)
          case Right(searchResult) ⇒
            Continuation(
              RangeCallbackReply(
                RangeCallbackReply.Reply.SubmitLeaf(
                  ReplySubmitLeaf(
                    searchResult match {
                      case Searching.Found(i) ⇒ ReplySubmitLeaf.SearchResult.Found(i)
                      case Searching.InsertionPoint(i) ⇒ ReplySubmitLeaf.SearchResult.InsertionPoint(i)
                    }
                  )
                )
              )
            )
        }

  }

  private def handleResult(
    implicit scheduler: Scheduler
  ): PartialFunction[RangeCallback.Callback, F[Flow[RangeCallbackReply]]] = {
    case ask if ask.isServerError ⇒
      val Some(err) = ask.serverError
      val serverError = ServerError(err.msg)
      // if server send an error we should close stream and lift error up
      Effect[F].raiseError[Flow[RangeCallbackReply]](serverError)
    case ask if ask.isValue ⇒
      val Some(RangeValue(key, value)) = ask._value
      logger.trace(s"DatasetStorageClient.range() received server value=$value for key=$key")
      Effect[F].pure(RangeResult(key.toByteArray, value.toByteArray))
  }

  /**
   * Run client bidi stream.
   *
   * @param pipe Bidi pipe for transport layer
   * @return
   */
  def runStream(
    handler: Observable[RangeCallbackReply] ⇒ IO[Observable[RangeCallback]]
  )(implicit sch: Scheduler): IO[Observable[(Array[Byte], Array[Byte])]] = {

    val subj = PublishSubject[RangeCallbackReply]()

    val requests = (
      Observable(
        RangeCallbackReply(
          RangeCallbackReply.Reply.DatasetInfo(DatasetInfo(ByteString.copyFrom(datasetId), version))
        )
      ) ++ subj
    ).map { el ⇒
      logger.trace(s"DatasetStorageClient.range() will send message to server $el")
      el
    }

    for {
      responses ← handler(requests)
    } yield {

      val mapped = responses.map {
        case RangeCallback(callback) ⇒
          logger.trace(s"DatasetStorageClient.range() received server ask=$callback")
          callback
      }

      val cycle = handleAsks(mapped).mapFuture {
        case c @ Continuation(reply) ⇒
          subj.onNext(reply).map(_ ⇒ Observable.empty)
        case er @ ErrorFromClient(err) ⇒
          subj.onNext(err).map(_ ⇒ Observable(er))
        case v => Future(Observable(v))
      }.flatten

      val result =
        cycle.flatMap {
          case ErrorFromClient(er) ⇒
            Observable.raiseError(ClientError(er.reply.clientError.get.msg))
          case RangeResult(k, v) ⇒
            logger.trace(s"DatasetStorageClient.range() received server value=$v for key=$k")
            Observable(k → v)
          case _ => Observable.empty
        }

      result
    }
  }

}

object ClientRange extends slogging.LazyLogging {

  /**
   * Prepare ''Range'' operation in remote MerkleBTree.
   *
   * @param datasetId Dataset ID
   * @param version Dataset version expected to the client
   * @param rangeCallbacks Wrapper for all callback needed for ''Range'' operation to the BTree
   * @return returns stream of found value.
   */
  def apply[F[_]: Effect](
    datasetId: Array[Byte],
    version: Long,
    rangeCallbacks: BTreeRpc.SearchCallback[F]
  ): ClientRange[F] = {
    new ClientRange(datasetId, version, rangeCallbacks)
  }
}
