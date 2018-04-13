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

package fluence.dataset.grpc.client

import cats.effect.Effect
import fluence.btree.protocol.BTreeRpc
import fluence.dataset._
import fluence.dataset.grpc.GrpcMonix._
import fluence.dataset.protocol.DatasetStorageRpc
import fluence.dataset.service.DatasetStorageRpcGrpc.DatasetStorageRpcStub
import fluence.transport.grpc.client.GrpcClient
import io.grpc.{CallOptions, ManagedChannel}
import monix.execution.Scheduler
import monix.reactive.{Observable, Pipe}

import scala.language.{higherKinds, implicitConversions}

/**
 * Clients implementation of [[DatasetStorageRpc]], allows talking to server via network.
 * All public methods called from the client side.
 * DatasetStorageClient initiates first request to server and then answered to server requests.
 *
 * @param stub Stub for calling server methods of [[DatasetStorageRpc]]
 * @tparam F A box for returning value
 */
class DatasetStorageClient[F[_]: Effect](
  stub: DatasetStorageRpcStub
)(implicit sch: Scheduler)
    extends DatasetStorageRpc[F, Observable] with slogging.LazyLogging {

  /**
   * Initiates ''Get'' operation in remote MerkleBTree.
   *
   * @param datasetId Dataset ID
   * @param version Dataset version expected to the client
   * @param getCallbacks Wrapper for all callback needed for ''Get'' operation to the BTree
   * @return returns found value, None if nothing was found.
   */
  override def get(
    datasetId: Array[Byte],
    version: Long,
    getCallbacks: BTreeRpc.SearchCallback[F]
  ): F[Option[Array[Byte]]] = {
    // Convert a remote stub call to monix pipe
    val pipe: Pipe[GetCallbackReply, GetCallback] = callToPipe(stub.get)

    Get(pipe, datasetId, version, getCallbacks)
  }

  /**
   * Initiates ''Range'' operation in remote MerkleBTree.
   *
   * @param datasetId Dataset ID
   * @param version Dataset version expected to the client
   * @param rangeCallbacks Wrapper for all callback needed for ''Range'' operation to the BTree
   * @return returns stream of found value.
   */
  override def range(
    datasetId: Array[Byte],
    version: Long,
    rangeCallbacks: BTreeRpc.SearchCallback[F]
  ): Observable[(Array[Byte], Array[Byte])] = {

    // Convert a remote stub call to monix pipe
    val pipe: Pipe[RangeCallbackReply, RangeCallback] = callToPipe(stub.range)

    fluence.dataset.grpc.client.Range(pipe, datasetId, version, rangeCallbacks)
  }

  /**
   * Initiates ''Put'' operation in remote MerkleBTree.
   *
   * @param datasetId Dataset ID
   * @param version Dataset version expected to the client
   * @param putCallbacks Wrapper for all callback needed for ''Put'' operation to the BTree.
   * @param encryptedValue Encrypted value.
   * @return returns old value if old value was overridden, None otherwise.
   */
  override def put(
    datasetId: Array[Byte],
    version: Long,
    putCallbacks: BTreeRpc.PutCallbacks[F],
    encryptedValue: Array[Byte]
  ): F[Option[Array[Byte]]] = {
    // Convert a remote stub call to monix pipe
    val pipe: Pipe[PutCallbackReply, PutCallback] = callToPipe(stub.put)

    Put(pipe, datasetId, version, putCallbacks, encryptedValue)
  }

  /**
   * Initiates ''Remove'' operation in remote MerkleBTree.
   *
   * @param datasetId Dataset ID
   * @param version Dataset version expected to the client
   * @param removeCallbacks Wrapper for all callback needed for ''Remove'' operation to the BTree.
   * @return returns old value that was deleted, None if nothing was deleted.
   */
  override def remove(
    datasetId: Array[Byte],
    version: Long,
    removeCallbacks: BTreeRpc.RemoveCallback[F]
  ): F[Option[Array[Byte]]] = ???
}

object DatasetStorageClient {

  /**
   * Shorthand to register [[DatasetStorageClient]] inside [[GrpcClient]].
   *
   * @param channel     Channel to remote node
   * @param callOptions Call options
   */
  def register[F[_]: Effect]()(
    channel: ManagedChannel,
    callOptions: CallOptions
  )(implicit scheduler: Scheduler): DatasetStorageRpc[F, Observable] =
    new DatasetStorageClient[F](new DatasetStorageRpcStub(channel, callOptions))
}
