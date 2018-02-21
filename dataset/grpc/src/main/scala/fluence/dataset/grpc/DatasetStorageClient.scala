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
      .transform(_.map { getCb ⇒
        val res = getCb.error → getCb.callback
        logger.trace(s"DatasetStorageClient.get() received server ask=$res")
        res
      })
      .multicast

    val handleAsks = serverAsk.collect { // Collect callbacks
      case tuple @ (_, ask) if ask.isDefined && !ask.isValue ⇒ tuple
    }.mapEval[F, GetCallbackReply.Reply] { // Route callbacks to ''getCallbacks''

      case (err, ask) if ask.isNextChildIndex ⇒ doIfNoErr(err) {
        val Some(nci) = ask.nextChildIndex

        getCallbacks
          .nextChildIndex(nci.keys.map(_.toByteArray).toArray, nci.childsChecksums.map(_.toByteArray).toArray)

          .map(i ⇒ ReplyNextChildIndex(i))
          .map(GetCallbackReply.Reply.NextChildIndex)
      }

      case (err, ask) if ask.isSubmitLeaf ⇒ doIfNoErr(err) {
        val Some(sl) = ask.submitLeaf

        getCallbacks
          .submitLeaf(sl.keys.map(_.toByteArray).toArray, sl.valuesChecksums.map(_.toByteArray).toArray)

          .map(oi ⇒ ReplySubmitLeaf(oi.getOrElse(-1)))
          .map(GetCallbackReply.Reply.SubmitLeaf)
      }
    }.map(r ⇒ GetCallbackReply(reply = r))

    (
      Observable(
        GetCallbackReply(
          reply = GetCallbackReply.Reply.DatasetId(DatasetId(ByteString.copyFrom(datasetId)))
        )
      ) ++ handleAsks
    ).subscribe(clientReply) // And clientReply response back to server

    val valueTask: Task[Option[Array[Byte]]] =
      serverAsk.collect { // Collect response value
        case tuple @ (_, ask) if ask.isValue ⇒ tuple
      }.map {
        case (err, ask) ⇒
          if (err.isDefined) {
            Task.raiseError(ServerError(err.get.msg))
          } else {
            Task {
              Option(ask._value.get.value)
                .filterNot(_.isEmpty)
                .map(_.toByteArray)
            }
          }
      }.headL.flatten // Take the first value, wrapped with Task

    run(valueTask)
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
      .transform(_.map { get ⇒
        val res = get.error → get.callback
        logger.trace(s"DatasetStorageClient.put() received server ask=$res")
        res
      })
      .multicast

    val handleAsks = serverAsk.collect { // Collect callbacks
      case tuple @ (_, ask) if ask.isDefined && !ask.isValue ⇒ tuple
    }.mapEval[F, PutCallbackReply.Reply] {

      case (err, ask) if ask.isNextChildIndex ⇒ doIfNoErr(err) {
        val Some(nci) = ask.nextChildIndex

        putCallbacks
          .nextChildIndex(nci.keys.map(_.toByteArray).toArray, nci.childsChecksums.map(_.toByteArray).toArray)

          .map(i ⇒ ReplyNextChildIndex(i))
          .map(PutCallbackReply.Reply.NextChildIndex)
      }

      case (err, ask) if ask.isPutDetails ⇒ doIfNoErr(err) {
        val Some(pd) = ask.putDetails

        putCallbacks
          .putDetails(pd.keys.map(_.toByteArray).toArray, pd.valuesChecksums.map(_.toByteArray).toArray)

          .map(cpd ⇒ ReplyPutDetails(
            key = ByteString.copyFrom(cpd.key),
            checksum = ByteString.copyFrom(cpd.valChecksum),
            searchResult = cpd.searchResult match {
              case Searching.Found(foundIndex)              ⇒ ReplyPutDetails.SearchResult.FoundIndex(foundIndex)
              case Searching.InsertionPoint(insertionPoint) ⇒ ReplyPutDetails.SearchResult.InsertionPoint(insertionPoint)
            }))
          .map(PutCallbackReply.Reply.PutDetails)
      }

      case (err, ask) if ask.isVerifyChanges ⇒ doIfNoErr(err) {
        val Some(vc) = ask.verifyChanges

        putCallbacks
          .verifyChanges(vc.serverMerkleRoot.toByteArray, vc.splitted)

          .map(_ ⇒ PutCallbackReply.Reply.VerifyChanges(ReplyVerifyChanges())) // TODO: here should be signature
      }

      case (err, ask) if ask.isChangesStored ⇒ doIfNoErr(err) {
        putCallbacks
          .changesStored()

          .map(_ ⇒ PutCallbackReply.Reply.ChangesStored(ReplyChangesStored()))
      }

    }.map(r ⇒ PutCallbackReply(reply = r))

    (
      Observable( // Pass the datasetId and value as the first, unasked pushes
        PutCallbackReply(
          reply = PutCallbackReply.Reply.DatasetId(DatasetId(ByteString.copyFrom(datasetId)))
        ),
        PutCallbackReply(
          reply = PutCallbackReply.Reply.Value(PutValue(ByteString.copyFrom(encryptedValue)))   // todo why ?
        )
      ) ++ handleAsks
    ).subscribe(clientReply) // And push response back to server

    val valueTask =
      serverAsk.collect { // Collect response value
        case tuple @ (_, ask) if ask.isValue ⇒ tuple
      }.map {
        case (err, ask) ⇒
          if (err.isDefined)
            Task.raiseError(ServerError(err.get.msg))
           else
            Task {
              Option(ask._value.get.value)
                .filterNot(_.isEmpty)
                .map(_.toByteArray)
            }
      }.headL.flatten // Take the first value, wrapped with Task

    run(valueTask)
  }

  /**
   * Initiates ''Remove'' operation in remote MerkleBTree.
   *
   * @param datasetId Dataset ID
   * @param removeCallbacks Wrapper for all callback needed for ''Remove'' operation to the BTree.
   * @return returns old value that was deleted, None if nothing was deleted.
   */
  override def remove(datasetId: Array[Byte], removeCallbacks: BTreeRpc.RemoveCallback[F]): F[Option[Array[Byte]]] = ???

  private def doIfNoErr[T](err: Option[Error])(action: ⇒ F[T]): F[T] =
    if (err.isDefined)
      Effect[F].raiseError[T](ServerError(err.get.msg))
    else
      action
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
  case class ServerError(error: String) extends NoStackTrace

}
