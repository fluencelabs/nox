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
import fluence.dataset.protocol.{ClientError, ServerError}
import fluence.dataset.protobuf._
import monix.eval.Task
import monix.execution.Scheduler
import monix.reactive.subjects.PublishSubject
import monix.reactive.Observable

import scala.collection.Searching
import scala.concurrent.Future
import scala.language.higherKinds

class ClientGet[F[_]: Effect](datasetId: Array[Byte], version: Long, getCallbacks: BTreeRpc.SearchCallback[F])
    extends slogging.LazyLogging {

  import DatasetClientUtils._

  private def handleClientErr(err: Throwable)(implicit sch: Scheduler): ErrorFromClient[GetCallbackReply] =
    ErrorFromClient(GetCallbackReply(GetCallbackReply.Reply.ClientError(Error(err.getMessage))))

  private def handleContinuation(
    implicit scheduler: Scheduler
  ): PartialFunction[GetCallback.Callback, F[Flow[GetCallbackReply]]] = {
    case ask if ask.isNextChildIndex ⇒
      val Some(nci) = ask.nextChildIndex

      getCallbacks
        .nextChildIndex(
          nci.keys.map(k ⇒ Key(k.toByteArray)).toArray,
          nci.childsChecksums.map(c ⇒ Hash(c.toByteArray)).toArray
        )
        .attempt
        .map {
          case Left(err) ⇒
            handleClientErr(err)
          case Right(idx) ⇒
            Continuation(GetCallbackReply(GetCallbackReply.Reply.NextChildIndex(ReplyNextChildIndex(idx))))
        }

    case ask if ask.isSubmitLeaf ⇒
      val Some(sl) = ask.submitLeaf

      getCallbacks
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
              GetCallbackReply(
                GetCallbackReply.Reply.SubmitLeaf(
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
  ): PartialFunction[GetCallback.Callback, F[Flow[GetCallbackReply]]] = {
    case ask if ask.isServerError ⇒
      val Some(err) = ask.serverError
      val serverError = ServerError(err.msg)
      // if server send an error we should close stream and lift error up
      Effect[F].raiseError(serverError)
    case ask if ask.isValue ⇒
      val Some(getValue) = ask._value
      logger.trace(s"DatasetStorageClient.get() received server value=$getValue")
      // if got success response or server error close stream and return value\error to user of this client
      Effect[F].pure(
        Result(
          Option(getValue.value)
            .filterNot(_.isEmpty)
            .map(_.toByteArray)
        )
      )
  }

  private def handleAsks(source: Observable[GetCallback.Callback])(
    implicit sch: Scheduler
  ): Observable[Flow[GetCallbackReply]] =
    source
      .mapEval[F, Flow[GetCallbackReply]] {
        handleContinuation orElse handleResult
      }

  /**
   * Initiate observable schema with pipe and run stream.
   *
   * @param pipe Bidi pipe for transport layer
   * @return returns found value, None if nothing was found.
   */
  def runStream(
    handler: Observable[GetCallbackReply] ⇒ IO[Observable[GetCallback]]
  )(implicit sch: Scheduler): IO[Option[Array[Byte]]] = {

    val subj = PublishSubject[GetCallbackReply]()

    val requests = (
      Observable(
        GetCallbackReply(
          GetCallbackReply.Reply.DatasetInfo(DatasetInfo(ByteString.copyFrom(datasetId), version))
        )
      ) ++ subj
    ).map { el =>
      logger.trace(s"DatasetStorageClient.get() will send message to server $el")
      el
    }

    for {
      responses ← handler(requests)
      cycle = {

        val mapped = responses.map {
          case GetCallback(callback) ⇒
            logger.trace(s"DatasetStorageClient.get() received server ask=$callback")
            callback
        }

        handleAsks(mapped).mapFuture {
          case c @ Continuation(reply) ⇒ subj.onNext(reply).map(_ ⇒ c)
          case res @ Result(v) ⇒ Future(res)
          case er @ ErrorFromClient(err) ⇒ subj.onNext(err).map(_ ⇒ er)
        }
      }

      result <- {
        cycle.concatMap {
          case r@Result(_) ⇒
            Observable(r, Stop)
          case er@ErrorFromClient(_) ⇒
            Observable(er, Stop)
          case c@Continuation(_) ⇒
            Observable(c)
        }.takeWhile {
          case Stop ⇒ false
          case _ ⇒ true
        }.lastOptionL
          .flatMap {
            case Some(ErrorFromClient(err)) ⇒
              Task.raiseError(ClientError(err.reply.clientError.get.msg))
            case Some(Result(v)) ⇒
              Task(v)
            case v ⇒
              logger.error("Unexpected message: " + v)
              Task.raiseError(new RuntimeException("Unexpected internal error"))
          }
          .toIO
      }
    } yield result
  }

}

object ClientGet {

  /**
   * Prepare ''Get'' operation in remote MerkleBTree.
   *
   * @param datasetId Dataset ID
   * @param version Dataset version expected to the client
   * @param getCallbacks Wrapper for all callback needed for ''Get'' operation to the BTree
   */
  def apply[F[_]: Effect](
    datasetId: Array[Byte],
    version: Long,
    getCallbacks: BTreeRpc.SearchCallback[F]
  ): ClientGet[F] = {

    new ClientGet(datasetId, version, getCallbacks)
  }
}
