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

import cats.effect.{Effect, IO}
import fluence.btree.protocol.BTreeRpc
import fluence.dataset.client.{ClientGet, ClientPut, ClientRange}
import fluence.dataset.protobuf._
import fluence.dataset.protocol.DatasetStorageRpc
import fluence.proxy.grpc.WebsocketMessage
import fluence.transport.websocket.GrpcProxyClient
import fluence.transport.websocket.WebsocketPipe.WebsocketClient
import monix.execution.Scheduler
import monix.reactive.{Observable, Observer, Pipe}

import scala.language.higherKinds

class DatasetStorageClient[F[_]: Effect](
  websocket: WebsocketClient[WebsocketMessage]
)(implicit sch: Scheduler)
    extends DatasetStorageRpc[F, Observable] with slogging.LazyLogging {

  import fluence.transport.websocket.ProtobufCodec._

  private val service = "fluence.dataset.protobuf.grpc.DatasetStorageRpc"

  def getPipe: Pipe[GetCallbackReply, GetCallback] = {

    val proxy =
      GrpcProxyClient.proxy(service, "get", websocket, generatedMessageCodec, protobufDynamicCodec(GetCallback))

    new Pipe[GetCallbackReply, GetCallback] {

      override def unicast: (Observer[GetCallbackReply], Observable[GetCallback]) = {
        (proxy.input, proxy.output)
      }
    }
  }

  val rangePipe: Pipe[RangeCallbackReply, RangeCallback] = new Pipe[RangeCallbackReply, RangeCallback] {
    override def unicast: (Observer[RangeCallbackReply], Observable[RangeCallback]) = {
      val proxy =
        GrpcProxyClient.proxy(service, "range", websocket, generatedMessageCodec, protobufDynamicCodec(RangeCallback))

      (proxy.input, proxy.output)
    }
  }

  val putPipe: Pipe[PutCallbackReply, PutCallback] = new Pipe[PutCallbackReply, PutCallback] {
    override def unicast: (Observer[PutCallbackReply], Observable[PutCallback]) = {
      val proxy =
        GrpcProxyClient.proxy(service, "put", websocket, generatedMessageCodec, protobufDynamicCodec(PutCallback))

      (proxy.input, proxy.output)
    }
  }

  /**
   * Initiates ''Get'' operation in remote MerkleBTree.
   *
   * @param datasetId       Dataset ID
   * @param version         Dataset version expected to the client
   * @param searchCallbacks Wrapper for all callback needed for ''Get'' operation to the BTree
   * @return returns found value, None if nothing was found.
   */
  override def get(
    datasetId: Array[Byte],
    version: Long,
    searchCallbacks: BTreeRpc.SearchCallback[F]
  ): IO[Option[Array[Byte]]] = ClientGet(datasetId, version, searchCallbacks).runStream(getPipe)

  /**
   * Initiates ''Range'' operation in remote MerkleBTree.
   *
   * @param datasetId       Dataset ID
   * @param version         Dataset version expected to the client
   * @param searchCallbacks Wrapper for all callback needed for ''Range'' operation to the BTree
   * @return returns stream of found value.
   */
  // TODO range request is not working for now for websockets
  override def range(
    datasetId: Array[Byte],
    version: Long,
    searchCallbacks: BTreeRpc.SearchCallback[F]
  ): Observable[(Array[Byte], Array[Byte])] =
    ??? //ClientRange(datasetId, version, searchCallbacks).runStream(rangePipe)

  /**
   * Initiates ''Put'' operation in remote MerkleBTree.
   *
   * @param datasetId      Dataset ID
   * @param version        Dataset version expected to the client
   * @param putCallbacks   Wrapper for all callback needed for ''Put'' operation to the BTree.
   * @param encryptedValue Encrypted value.
   * @return returns old value if old value was overridden, None otherwise.
   */
  override def put(
    datasetId: Array[Byte],
    version: Long,
    putCallbacks: BTreeRpc.PutCallbacks[F],
    encryptedValue: Array[Byte]
  ): IO[Option[Array[Byte]]] = ClientPut(datasetId, version, putCallbacks, encryptedValue).runStream(putPipe)

  /**
   * Initiates ''Remove'' operation in remote MerkleBTree.
   *
   * @param datasetId       Dataset ID
   * @param version         Dataset version expected to the client
   * @param removeCallbacks Wrapper for all callback needed for ''Remove'' operation to the BTree.
   * @return returns old value that was deleted, None if nothing was deleted.
   */
  override def remove(
    datasetId: Array[Byte],
    version: Long,
    removeCallbacks: BTreeRpc.RemoveCallback[F]
  ): IO[Option[Array[Byte]]] = ???
}
