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
import cats.syntax.functor._
import com.google.protobuf.ByteString
import fluence.btree.protocol.BTreeRpc
import fluence.dataset.grpc.DatasetStorageClient.ServerError
import fluence.dataset.grpc.GrpcMonix._
import fluence.dataset.grpc.storage.DatasetStorageRpcGrpc.DatasetStorageRpcStub
import fluence.dataset.grpc.storage._
import fluence.dataset.protocol.storage.DatasetStorageRpc
import fluence.transport.grpc.client.GrpcClient
import io.grpc.{ CallOptions, ManagedChannel }
import monix.eval.Task
import monix.execution.Scheduler
import monix.reactive.Observable

import scala.collection.Searching
import scala.language.{ higherKinds, implicitConversions }
import scala.util.control.NoStackTrace

class DatasetStorageClient[F[_] : Effect](
    stub: DatasetStorageRpcStub
)(implicit sch: Scheduler) extends DatasetStorageRpc[F] with slogging.LazyLogging {

  private def run[A](fa: Task[A]): F[A] = fa.toIO.to[F]

  /**
   * Initiates ''Get'' operation in remote MerkleBTree.
   *
   * @param datasetId    Dataset ID
   * @param getCallbacks Wrapper for all callback needed for ''Get'' operation to the BTree
   * @return returns found value, None if nothing was found.
   */
  override def get(datasetId: Array[Byte], getCallbacks: BTreeRpc.GetCallbacks[F]): F[Option[Array[Byte]]] = {
    // Convert a remote stub call to monix pipe
    val pipe = callToPipe(stub.get)

    // Get observer/observable for request's bidiflow
    val (clientReply, serverAsk) = pipe
      .transform(_.map {
        case GetCallback(callback) ⇒
          logger.trace(s"DatasetStorageClient.get() received server ask=$callback")
          callback
      })
      .multicast

    val handleAsks = serverAsk
      .collect { case ask if ask.isDefined && !ask.isValue && !ask.isServerError ⇒ ask } // Collect callbacks
      .mapEval[F, GetCallbackReply] {

        case ask if ask.isNextChildIndex ⇒
          val Some(nci) = ask.nextChildIndex

          getCallbacks
            .nextChildIndex(nci.keys.map(_.toByteArray).toArray, nci.childsChecksums.map(_.toByteArray).toArray)
            .attempt
            .map {
              case Left(err) ⇒
                GetCallbackReply(GetCallbackReply.Reply.ClientError(Error(err.getMessage)))
              case Right(idx) ⇒
                GetCallbackReply(GetCallbackReply.Reply.NextChildIndex(ReplyNextChildIndex(idx)))
            }

        case ask if ask.isSubmitLeaf ⇒
          val Some(sl) = ask.submitLeaf

          getCallbacks
            .submitLeaf(sl.keys.map(_.toByteArray).toArray, sl.valuesChecksums.map(_.toByteArray).toArray)
            .attempt
            .map {
              case Left(err) ⇒
                GetCallbackReply(GetCallbackReply.Reply.ClientError(Error(err.getMessage)))
              case Right(idx) ⇒
                GetCallbackReply(GetCallbackReply.Reply.SubmitLeaf(ReplySubmitLeaf(idx.getOrElse(-1))))
            }

      }

    (
      Observable(
        GetCallbackReply(
          GetCallbackReply.Reply.DatasetId(DatasetId(ByteString.copyFrom(datasetId)))
        )
      ) ++ handleAsks
    ).subscribe(clientReply) // And clientReply response back to server

    val errorOrValue =
      serverAsk
        .collect {  // Collect terminal task with value/error
          case ask if ask.isServerError ⇒
            val Some(err) = ask.serverError
            Task.raiseError(ServerError(err.msg))
          case ask if ask.isValue ⇒
            val Some(getValue) = ask._value
            Task {
              Option(getValue.value)
                .filterNot(_.isEmpty)
                .map(_.toByteArray)
            }
        }.headL.flatten // Take the first value, wrapped with Task

    run(errorOrValue)
  }

  /**
   * Initiates ''Put'' operation in remote MerkleBTree.
   *
   * @param datasetId      Dataset ID
   * @param putCallbacks   Wrapper for all callback needed for ''Put'' operation to the BTree.
   * @param encryptedValue Encrypted value.
   * @return returns old value if old value was overridden, None otherwise.
   */
  override def put(datasetId: Array[Byte], putCallbacks: BTreeRpc.PutCallbacks[F], encryptedValue: Array[Byte]): F[Option[Array[Byte]]] = {
    // Convert a remote stub call to monix pipe
    val pipe = callToPipe(stub.put)

    val (clientReply, serverAsk) = pipe
      .transform(_.map {
        case PutCallback(callback) ⇒
          logger.trace(s"DatasetStorageClient.put() received server ask=$callback")
          callback
      })
      .multicast

    val handleAsks = serverAsk
      .collect { case ask if ask.isDefined && !ask.isValue ⇒ ask } // Collect callbacks
      .mapEval[F, PutCallbackReply] {

        case ask if ask.isNextChildIndex ⇒
          val Some(nci) = ask.nextChildIndex

          putCallbacks
            .nextChildIndex(nci.keys.map(_.toByteArray).toArray, nci.childsChecksums.map(_.toByteArray).toArray)
            .attempt
            .map {
              case Left(err) ⇒
                PutCallbackReply(PutCallbackReply.Reply.ClientError(Error(err.getMessage)))
              case Right(idx) ⇒
                PutCallbackReply(PutCallbackReply.Reply.NextChildIndex(ReplyNextChildIndex(idx)))
            }

        case ask if ask.isPutDetails ⇒
          val Some(pd) = ask.putDetails

          putCallbacks
            .putDetails(pd.keys.map(_.toByteArray).toArray, pd.valuesChecksums.map(_.toByteArray).toArray)
            .attempt
            .map {
              case Left(err) ⇒
                PutCallbackReply(PutCallbackReply.Reply.ClientError(Error(err.getMessage)))
              case Right(cpd) ⇒
                val putDetails = ReplyPutDetails(
                  key = ByteString.copyFrom(cpd.key),
                  checksum = ByteString.copyFrom(cpd.valChecksum),
                  searchResult = cpd.searchResult match {
                    case Searching.Found(foundIndex)              ⇒ ReplyPutDetails.SearchResult.FoundIndex(foundIndex)
                    case Searching.InsertionPoint(insertionPoint) ⇒ ReplyPutDetails.SearchResult.InsertionPoint(insertionPoint)
                  })
                PutCallbackReply(PutCallbackReply.Reply.PutDetails(putDetails))
            }

        case ask if ask.isVerifyChanges ⇒
          val Some(vc) = ask.verifyChanges

          putCallbacks
            .verifyChanges(vc.serverMerkleRoot.toByteArray, vc.splitted)
            .attempt
            .map {
              case Left(err) ⇒
                PutCallbackReply(PutCallbackReply.Reply.ClientError(Error(err.getMessage)))
              case Right(idx) ⇒
                PutCallbackReply(PutCallbackReply.Reply.VerifyChanges(ReplyVerifyChanges())) // TODO: here should be signature
            }

        case ask if ask.isChangesStored ⇒
          putCallbacks
            .changesStored()
            .attempt
            .map {
              case Left(err) ⇒
                PutCallbackReply(PutCallbackReply.Reply.ClientError(Error(err.getMessage)))
              case Right(idx) ⇒
                PutCallbackReply(PutCallbackReply.Reply.ChangesStored(ReplyChangesStored()))
            }
      }

    (
      Observable( // Pass the datasetId and value as the first, unasked pushes
        PutCallbackReply(
          reply = PutCallbackReply.Reply.DatasetId(DatasetId(ByteString.copyFrom(datasetId)))
        ),
        PutCallbackReply(
          reply = PutCallbackReply.Reply.Value(PutValue(ByteString.copyFrom(encryptedValue)))
        )
      ) ++ handleAsks
    ).subscribe(clientReply) // And push response back to server

    val errorOrValue =
      serverAsk
        .collect {  // Collect terminal task with value/error
          case ask if ask.isServerError ⇒
            val Some(err) = ask.serverError
            Task.raiseError(ServerError(err.msg))
          case ask if ask.isValue ⇒
            val Some(getValue) = ask._value
            Task {
              Option(getValue.value)
                .filterNot(_.isEmpty)
                .map(_.toByteArray)
            }
        }.headL.flatten // Take the first value, wrapped with Task

    run(errorOrValue)

  }

  /**
   * Initiates ''Remove'' operation in remote MerkleBTree.
   *
   * @param datasetId Dataset ID
   * @param removeCallbacks Wrapper for all callback needed for ''Remove'' operation to the BTree.
   * @return returns old value that was deleted, None if nothing was deleted.
   */
  override def remove(datasetId: Array[Byte], removeCallbacks: BTreeRpc.RemoveCallback[F]): F[Option[Array[Byte]]] = ???

}

object DatasetStorageClient {

  /**
   * Shorthand to register [[DatasetStorageClient]] inside [[GrpcClient]].
   *
   * @param channel     Channel to remote node
   * @param callOptions Call options
   */
  def register[F[_] : Effect]()(
    channel: ManagedChannel,
    callOptions: CallOptions
  )(implicit scheduler: Scheduler): DatasetStorageRpc[F] =
    new DatasetStorageClient[F](new DatasetStorageRpcStub(channel, callOptions))

  /**  Error from server(node) side. */
  case class ServerError(msg: String) extends NoStackTrace {
    override def getMessage: String = msg
  }

}
