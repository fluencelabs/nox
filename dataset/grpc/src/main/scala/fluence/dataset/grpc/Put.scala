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

package fluence.dataset.grpc

import cats.effect.Effect
import cats.syntax.applicativeError._
import cats.syntax.flatMap._
import com.google.protobuf.ByteString
import fluence.btree.core.{Hash, Key}
import fluence.btree.protocol.BTreeRpc
import fluence.dataset.grpc.DatasetStorageClient.ServerError
import fluence.dataset.grpc.DatasetStorageServer.ClientError
import monix.eval.{MVar, Task}
import monix.execution.Scheduler
import monix.reactive.{Observable, Pipe}

import scala.collection.Searching

class Put[F[_]: Effect] private ()(implicit sch: Scheduler) extends slogging.LazyLogging {

  import DatasetOperation._

  /**
   * Initiates ''Put'' operation in remote MerkleBTree.
   *
   * @param datasetId Dataset ID
   * @param version Dataset version expected to the client
   * @param putCallbacks Wrapper for all callback needed for ''Put'' operation to the BTree.
   * @param encryptedValue Encrypted value.
   * @return returns old value if old value was overridden, None otherwise.
   */
  def put(
    pipe: Pipe[PutCallbackReply, PutCallback],
    datasetId: Array[Byte],
    version: Long,
    putCallbacks: BTreeRpc.PutCallbacks[F],
    encryptedValue: Array[Byte]
  ): F[Option[Array[Byte]]] = {

    val (pushClientReply, pullServerAsk) = pipe
      .transform(_.map {
        case PutCallback(callback) ⇒
          logger.trace(s"DatasetStorageClient.put() received server ask=$callback")
          callback
      })
      .multicast

    val clientError = MVar.empty[ClientError].memoize

    /** Puts error to client error(for returning error to user of this client), and return reply with error for server.*/
    def handleClientErr(err: Throwable): F[PutCallbackReply] =
      run(
        for {
          ce ← clientError
          _ ← ce.put(ClientError(err.getMessage))
        } yield PutCallbackReply(PutCallbackReply.Reply.ClientError(Error(err.getMessage)))
      )

    val handleAsks = pullServerAsk.collect { case ask if ask.isDefined && !ask.isValue ⇒ ask } // Collect callbacks
      .mapEval[F, PutCallbackReply] {

        case ask if ask.isNextChildIndex ⇒
          val Some(nci) = ask.nextChildIndex

          putCallbacks
            .nextChildIndex(
              nci.keys.map(k ⇒ Key(k.toByteArray)).toArray,
              nci.childsChecksums.map(c ⇒ Hash(c.toByteArray)).toArray
            )
            .attempt
            .flatMap {
              case Left(err) ⇒
                handleClientErr(err)
              case Right(idx) ⇒
                Effect[F].pure(PutCallbackReply(PutCallbackReply.Reply.NextChildIndex(ReplyNextChildIndex(idx))))
            }

        case ask if ask.isPutDetails ⇒
          val Some(pd) = ask.putDetails

          putCallbacks
            .putDetails(
              pd.keys.map(k ⇒ Key(k.toByteArray)).toArray,
              pd.valuesChecksums.map(c ⇒ Hash(c.toByteArray)).toArray
            )
            .attempt
            .flatMap {
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
                Effect[F].pure(PutCallbackReply(PutCallbackReply.Reply.PutDetails(putDetails)))
            }

        case ask if ask.isVerifyChanges ⇒
          val Some(vc) = ask.verifyChanges

          putCallbacks
            .verifyChanges(Hash(vc.serverMerkleRoot.toByteArray), vc.splitted)
            .attempt
            .flatMap {
              case Left(err) ⇒
                handleClientErr(err)
              case Right(signature) ⇒
                Effect[F].pure(
                  PutCallbackReply(
                    PutCallbackReply.Reply.VerifyChanges(ReplyVerifyChanges(ByteString.copyFrom(signature.toArray)))
                  )
                )
            }

        case ask if ask.isChangesStored ⇒
          putCallbacks
            .changesStored()
            .attempt
            .flatMap {
              case Left(err) ⇒
                handleClientErr(err)
              case Right(idx) ⇒
                Effect[F].pure(PutCallbackReply(PutCallbackReply.Reply.ChangesStored(ReplyChangesStored())))
            }
      }

    (
      Observable( // Pass the datasetId and value as the first, unasked pushes
        PutCallbackReply(
          PutCallbackReply.Reply.DatasetInfo(DatasetInfo(ByteString.copyFrom(datasetId), version))
        ),
        PutCallbackReply(
          PutCallbackReply.Reply.Value(PutValue(ByteString.copyFrom(encryptedValue)))
        )
      ) ++ handleAsks
    ).subscribe(pushClientReply) // And push response back to server

    val serverErrOrVal =
      pullServerAsk.collect { // Collect terminal task with value/error
        case ask if ask.isServerError ⇒
          val Some(err) = ask.serverError
          val serverError = ServerError(err.msg)
          // if server send the error we should close stream and lift error up
          Task(pushClientReply.onError(serverError))
            .flatMap(_ ⇒ Task.raiseError[Option[Array[Byte]]](serverError))
        case ask if ask.isValue ⇒
          val Some(getValue) = ask._value
          // if got success response or server error close stream and return value\error to user of this client
          Task(pushClientReply.onComplete()).map { _ ⇒
            Option(getValue.value)
              .filterNot(_.isEmpty)
              .map(_.toByteArray)
          }
      }.headOptionL // Take the first option value or server error

    composeResult(clientError, serverErrOrVal)

  }
}

object Put {

  def apply[F[_]: Effect](
    pipe: Pipe[PutCallbackReply, PutCallback],
    datasetId: Array[Byte],
    version: Long,
    putCallbacks: BTreeRpc.PutCallbacks[F],
    encryptedValue: Array[Byte]
  )(implicit sch: Scheduler): F[Option[Array[Byte]]] =
    new Put().put(pipe, datasetId, version, putCallbacks, encryptedValue)
}
