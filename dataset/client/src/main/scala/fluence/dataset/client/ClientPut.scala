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

trait FlowResult
case class Continuation(reply: PutCallbackReply) extends FlowResult
case class Result(result: Option[Array[Byte]]) extends FlowResult
case class ErrorFromClient(reply: PutCallbackReply) extends FlowResult
case object Stop extends FlowResult


class ClientPut[F[_]: Effect](
  datasetId: Array[Byte],
  version: Long,
  putCallbacks: BTreeRpc.PutCallbacks[F],
  encryptedValue: Array[Byte]
) extends slogging.LazyLogging {

  private def handleClientErr(err: Throwable): ErrorFromClient = {
    ErrorFromClient(PutCallbackReply(PutCallbackReply.Reply.ClientError(Error(err.getMessage))))
  }

  private def handleContinuation(implicit scheduler: Scheduler): PartialFunction[PutCallback.Callback, F[FlowResult]] = {
    case ask if ask.isNextChildIndex ⇒
      val Some(nci) = ask.nextChildIndex

      putCallbacks
        .nextChildIndex(
          nci.keys.map(k ⇒ Key(k.toByteArray)).toArray,
          nci.childsChecksums.map(c ⇒ Hash(c.toByteArray)).toArray
        )
        .attempt
        .map {
          case Left(err) ⇒
            handleClientErr(err)
          case Right(idx) ⇒
            Continuation(PutCallbackReply(PutCallbackReply.Reply.NextChildIndex(ReplyNextChildIndex(idx))))
        }

    case ask if ask.isPutDetails ⇒
      val Some(pd) = ask.putDetails

      putCallbacks
        .putDetails(
          pd.keys.map(k ⇒ Key(k.toByteArray)).toArray,
          pd.valuesChecksums.map(c ⇒ Hash(c.toByteArray)).toArray
        )
        .attempt
        .map {
          case Left(err) ⇒
            handleClientErr(err)
          case Right(cpd) ⇒
            val putDetails = ReplyPutDetails(
              key = ByteString.copyFrom(cpd.key.bytes),
              checksum = ByteString.copyFrom(cpd.valChecksum.bytes),
              searchResult = cpd.searchResult match {
                case Searching.Found(foundIndex) ⇒ ReplyPutDetails.SearchResult.Found(foundIndex)
                case Searching.InsertionPoint(insertionPoint) ⇒
                  ReplyPutDetails.SearchResult.InsertionPoint(insertionPoint)
              }
            )
            Continuation(PutCallbackReply(PutCallbackReply.Reply.PutDetails(putDetails)))
        }

    case ask if ask.isVerifyChanges ⇒
      val Some(vc) = ask.verifyChanges

      putCallbacks
        .verifyChanges(Hash(vc.serverMerkleRoot.toByteArray), vc.splitted)
        .attempt
        .map {
          case Left(err) ⇒
            handleClientErr(err)
          case Right(signature) ⇒
            Continuation(PutCallbackReply(
              PutCallbackReply.Reply.VerifyChanges(ReplyVerifyChanges(ByteString.copyFrom(signature.toArray)))
            ))

        }

    case ask if ask.isChangesStored ⇒
      putCallbacks
        .changesStored()
        .attempt
        .map {
          case Left(err) ⇒
            handleClientErr(err)
          case Right(idx) ⇒
            Continuation(PutCallbackReply(PutCallbackReply.Reply.ChangesStored(ReplyChangesStored())))
        }
  }

  private def handleResult(
    implicit scheduler: Scheduler
  ): PartialFunction[PutCallback.Callback, F[FlowResult]] = {
    case ask if ask.isServerError ⇒
      val Some(err) = ask.serverError
      val serverError = ServerError(err.msg)
      // if server send the error we should close stream and lift error up
      Effect[F].raiseError[FlowResult](serverError)
    case ask if ask.isValue ⇒
      val Some(getValue) = ask._value
      logger.trace(s"DatasetStorageClient.put() received server value=$getValue")
      // if got success response or server error close stream and return value\error to user of this client
      Effect[F].pure(
        Result(
          Option(getValue.value)
            .filterNot(_.isEmpty)
            .map(_.toByteArray)
        )
      )
  }

  private def handleAsks(source: Observable[PutCallback.Callback])(
    implicit sch: Scheduler
  ): Observable[FlowResult] =
    source
      .mapEval[F, FlowResult] {
        handleContinuation orElse handleResult
      }

  /**
   *
   * @param handler
   * @param sch
   * @return
   */
  def runStream(
    handler: Observable[PutCallbackReply] ⇒ IO[Observable[PutCallback]]
  )(implicit sch: Scheduler): IO[Option[Array[Byte]]] = {

    val subj = PublishSubject[PutCallbackReply]()

    def requests: Observable[PutCallbackReply] =
      (Observable( // Pass the datasetId and value as the first, unasked pushes
        PutCallbackReply(
          PutCallbackReply.Reply.DatasetInfo(DatasetInfo(ByteString.copyFrom(datasetId), version))
        ),
        PutCallbackReply(
          PutCallbackReply.Reply.Value(PutValue(ByteString.copyFrom(encryptedValue)))
        )
      ) ++ subj).map { el =>
        logger.trace(s"DatasetStorageClient.put() will send message to server $el")
        el
      }

    for {
      responses ← handler(requests)
      cycle = {

        val mapped = responses.map {
          case PutCallback(callback) ⇒
            logger.trace(s"DatasetStorageClient.put() received server ask=$callback")
            callback
        }

        handleAsks(mapped)
          .mapFuture {
            case c@Continuation(reply) => subj.onNext(reply).map(_ ⇒ c)
            case res@Result(v) => Future(res)
            case er@ErrorFromClient(err) => subj.onNext(err).map(_ ⇒ er)
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

object ClientPut extends slogging.LazyLogging {

  /**
   * Initiates ''Put'' operation in remote MerkleBTree.
   *

   * @param datasetId Dataset ID
   * @param version Dataset version expected to the client
   * @param putCallbacks Wrapper for all callback needed for ''Put'' operation to the BTree.
   * @param encryptedValue Encrypted value.
   * @return returns old value if old value was overridden, None otherwise.
   */
  def apply[F[_]: Effect](
    datasetId: Array[Byte],
    version: Long,
    putCallbacks: BTreeRpc.PutCallbacks[F],
    encryptedValue: Array[Byte]
  ): ClientPut[F] = {
    new ClientPut(datasetId, version, putCallbacks, encryptedValue)
  }
}
