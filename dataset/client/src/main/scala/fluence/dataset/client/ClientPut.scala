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
import monix.reactive.{Observable, Observer}

import scala.collection.Searching
import scala.concurrent.Future
import scala.language.higherKinds

class ClientPut[F[_]: Effect](
  datasetId: Array[Byte],
  version: Long,
  putCallbacks: BTreeRpc.PutCallbacks[F],
  encryptedValue: Array[Byte]
) extends slogging.LazyLogging {

  /** Puts error to client error(for returning error to user of this client), and return reply with error for server.*/
  private def handleClientErr(err: Throwable): PutCallbackReply =
    PutCallbackReply(PutCallbackReply.Reply.ClientError(Error(err.getMessage)))

  type Results = Either[Option[Array[Byte]], PutCallbackReply]

  def handler(implicit scheduler: Scheduler): PartialFunction[PutCallback.Callback, F[PutCallbackReply]] = {
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
            PutCallbackReply(PutCallbackReply.Reply.NextChildIndex(ReplyNextChildIndex(idx)))
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
            PutCallbackReply(PutCallbackReply.Reply.PutDetails(putDetails))
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
            PutCallbackReply(
              PutCallbackReply.Reply.VerifyChanges(ReplyVerifyChanges(ByteString.copyFrom(signature.toArray)))
            )

        }

    case ask if ask.isChangesStored ⇒
      putCallbacks
        .changesStored()
        .attempt
        .map {
          case Left(err) ⇒
            handleClientErr(err)
          case Right(idx) ⇒
            PutCallbackReply(PutCallbackReply.Reply.ChangesStored(ReplyChangesStored()))
        }
  }

  def handleCont(
    implicit scheduler: Scheduler
  ): PartialFunction[PutCallback.Callback, F[Results]] = {
    handler andThen (_.map(Right.apply[Option[Array[Byte]], PutCallbackReply]))
  }

  def handleResult(
    implicit scheduler: Scheduler
  ): PartialFunction[PutCallback.Callback, F[Results]] = {
    case ask if ask.isServerError ⇒
      val Some(err) = ask.serverError
      val serverError = ServerError(err.msg)
      // if server send the error we should close stream and lift error up
      Effect[F].raiseError[Results](serverError)
    case ask if ask.isValue ⇒
      val Some(getValue) = ask._value
      logger.trace(s"DatasetStorageClient.put() received server value=$getValue")
      // if got success response or server error close stream and return value\error to user of this client
      Effect[F].pure(
        Left[Option[Array[Byte]], PutCallbackReply](
          Option(getValue.value)
            .filterNot(_.isEmpty)
            .map(_.toByteArray)
        )
      )
  }

  private def handleAsks(source: Observable[PutCallback.Callback])(
    implicit sch: Scheduler
  ): Observable[Results] =
    source
      .mapEval[F, Results] {
        handleCont orElse handleResult
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
      Observable( // Pass the datasetId and value as the first, unasked pushes
        PutCallbackReply(
          PutCallbackReply.Reply.DatasetInfo(DatasetInfo(ByteString.copyFrom(datasetId), version))
        ),
        PutCallbackReply(
          PutCallbackReply.Reply.Value(PutValue(ByteString.copyFrom(encryptedValue)))
        )
      ) ++ subj

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
            case r @ Right(v) ⇒ subj.onNext(v).map(_ ⇒ r)
            case l @ Left(_) ⇒
              subj.onComplete()
              Future(l)
          }
      }

      result <- {
        cycle.concatMap {
          case l @ Left(_) ⇒
            Observable(Some(l), None)
          case r @ Right(v) if v.reply.isClientError ⇒
            Observable(Some(r), None)
          case v ⇒
            Observable(Some(v))
        }.takeWhile {
          case Some(_) ⇒ true
          case None ⇒ false
        }.lastOptionL.map(_.flatten)
          .flatMap {
            case Some(Right(v)) if v.reply.isClientError ⇒
              Task.raiseError(ClientError(v.reply.clientError.get.msg))
            case Some(Left(v)) ⇒
              Task(v)
            case v ⇒
              println("LAST VALUE === " + v)
              Task.raiseError(new Exception("unexpected"))
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
