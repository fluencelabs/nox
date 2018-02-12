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
import cats.{ MonadError, ~> }
import com.google.protobuf.ByteString
import fluence.btree.protocol.BTreeRpc
import fluence.dataset.grpc.GrpcMonix._
import fluence.dataset.grpc.storage.DatasetStorageRpcGrpc.DatasetStorageRpcStub
import fluence.dataset.grpc.storage._
import fluence.dataset.protocol.storage.DatasetStorageRpc
import fluence.transport.grpc.client.GrpcClient
import io.grpc.{ CallOptions, ManagedChannel }
import monix.eval.Task
import monix.execution.Scheduler.Implicits.global
import monix.reactive.Observable

import scala.collection.Searching
import scala.language.{ higherKinds, implicitConversions }

class DatasetStorageClient[F[_]](stub: DatasetStorageRpcStub)(implicit F: MonadError[F, Throwable], run: Task ~> F, Eff: Effect[F]) extends DatasetStorageRpc[F] {
  /**
   * Initiates ''Get'' operation in remote MerkleBTree.
   *
   * @param getCallbacks Wrapper for all callback needed for ''Get'' operation to the BTree
   * @return returns found value, None if nothing was found.
   */
  override def get(datasetId: Array[Byte], getCallbacks: BTreeRpc.GetCallbacks[F]): F[Option[Array[Byte]]] = {
    // Convert a remote stub call to monix pipe
    val pipe = callToPipe(stub.get)

    // Get observer/observable for request's bidiflow
    val (push, pull) = pipe
      .transform(_.map(_.callback))
      .multicast

    val handleAsks = pull.collect { // Collect callbacks
      case ask if ask.isDefined && !ask.isValue ⇒ ask
    }.mapEval[F, GetCallbackReply.Reply]{ // Route callbacks to ''getCallbacks''

      case ask if ask.isNextChildIndex ⇒
        val Some(nci) = ask.nextChildIndex

        getCallbacks
          .nextChildIndex(nci.keys.map(_.toByteArray).toArray, nci.childsChecksums.map(_.toByteArray).toArray)

          .map(i ⇒ ReplyNextChildIndex(i))
          .map(GetCallbackReply.Reply.NextChildIndex)

      case ask if ask.isSubmitLeaf ⇒
        val Some(sl) = ask.submitLeaf

        getCallbacks
          .submitLeaf(sl.keys.map(_.toByteArray).toArray, sl.valuesChecksums.map(_.toByteArray).toArray)

          .map(oi ⇒ ReplySubmitLeaf(oi.getOrElse(-1)))
          .map(GetCallbackReply.Reply.SubmitLeaf)

    }.map(GetCallbackReply(_))

    (
      Observable(
        GetCallbackReply(GetCallbackReply.Reply.DatasetId(
          DatasetId(ByteString.copyFrom(datasetId))
        ))
      ) ++ handleAsks
    ).subscribe(push) // And push response back to server

    val value = pull.collect { // Collect response value
      case gv if gv.isValue ⇒ gv._value.get
    }.map(g ⇒ // Response value is an optional byte array
      Option(g.value)
        .filterNot(_.isEmpty)
        .map(_.toByteArray)
    )
      .headL // Take the first value, wrapped with Task

    run(value)
  }

  /**
   * Initiates ''Put'' operation in remote MerkleBTree.
   *
   * @param putCallbacks    Wrapper for all callback needed for ''Put'' operation to the BTree.
   * @param encryptedValue Encrypted value.
   * @return returns old value if old value was overridden, None otherwise.
   */
  override def put(datasetId: Array[Byte], putCallbacks: BTreeRpc.PutCallbacks[F], encryptedValue: Array[Byte]): F[Option[Array[Byte]]] = {
    // Convert a remote stub call to monix pipe
    val pipe = callToPipe(stub.put)

    // Get observer/observable for request's bidiflow
    val (push, pull) = pipe
      .transform(_.map(_.callback))
      .multicast

    val handleAsks = pull.collect { // Collect callbacks
      case ask if ask.isDefined && !ask.isValue ⇒ ask
    }.mapEval[F, PutCallbackReply.Reply]{

      case ask if ask.isNextChildIndex ⇒
        val Some(nci) = ask.nextChildIndex

        putCallbacks
          .nextChildIndex(nci.keys.map(_.toByteArray).toArray, nci.childsChecksums.map(_.toByteArray).toArray)

          .map(i ⇒ ReplyNextChildIndex(i))
          .map(PutCallbackReply.Reply.NextChildIndex)

      case ask if ask.isPutDetails ⇒
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

      case ask if ask.isVerifyChanges ⇒
        val Some(vc) = ask.verifyChanges

        putCallbacks
          .verifyChanges(vc.serverMerkleRoot.toByteArray, vc.splitted)

          .map(_ ⇒ PutCallbackReply.Reply.VerifyChanges(ReplyVerifyChanges()))

      case ask if ask.isChangesStored ⇒

        putCallbacks
          .changesStored()

          .map(_ ⇒ PutCallbackReply.Reply.ChangesStored(ReplyChangesStored()))

    }.map(PutCallbackReply(_))

    (
      Observable( // Pass the datasetId and value as the first, unasked pushes
        PutCallbackReply(PutCallbackReply.Reply.DatasetId(
          DatasetId(ByteString.copyFrom(datasetId)))
        ),
        PutCallbackReply(PutCallbackReply.Reply.Value(
          PutValue(ByteString.copyFrom(encryptedValue)))
        )
      )
        ++ handleAsks
    ).subscribe(push) // And push response back to server

    val value = pull.collect { // Collect response value
      case gv if gv.isValue ⇒ gv._value.get
    }.map(g ⇒ // Response value is an optional byte array
      Option(g.value)
        .filterNot(_.isEmpty)
        .map(_.toByteArray)
    )
      .headL // Take the first value, wrapped with Task

    run(value)
  }

  /**
   * Initiates ''Remove'' operation in remote MerkleBTree.
   *
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
  def register[F[_]]()(
    channel: ManagedChannel,
    callOptions: CallOptions
  )(implicit run: Task ~> F, eff: Effect[F], F: MonadError[F, Throwable]): DatasetStorageRpc[F] = {
    new DatasetStorageClient[F](new DatasetStorageRpcStub(channel, callOptions))
  }
}
