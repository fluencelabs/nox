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
import fluence.transport.websocket.{GrpcProxyClient, WebsocketPipe}
import monix.execution.Scheduler
import monix.reactive.{Observable, Observer, Pipe}

import scala.language.higherKinds

/**
 * Client for interaction with the database.
 *
 * @param connection Websocket pipe.
 */
class DatasetStorageClient[F[_]: Effect](connection: IO[WebsocketPipe[WebsocketMessage, WebsocketMessage]])(
  implicit sch: Scheduler
) extends DatasetStorageRpc[F, Observable] with slogging.LazyLogging {

  import fluence.transport.websocket.ProtobufCodec._

  private val service = "fluence.dataset.protobuf.grpc.DatasetStorageRpc"

  def getPipe: IO[Pipe[GetCallbackReply, GetCallback]] = {
    for {
      websocket ← connection
      proxy = GrpcProxyClient.proxy(service, "get", websocket, generatedMessageCodec, protobufDynamicCodec(GetCallback))
    } yield {
      new Pipe[GetCallbackReply, GetCallback] {

        override def unicast: (Observer[GetCallbackReply], Observable[GetCallback]) = {
          (proxy.input, proxy.output)
        }
      }
    }
  }

  val rangePipe: IO[Pipe[RangeCallbackReply, RangeCallback]] =
    for {
      websocket ← connection
      proxy = GrpcProxyClient
        .proxy(service, "range", websocket, generatedMessageCodec, protobufDynamicCodec(RangeCallback))
    } yield {
      new Pipe[RangeCallbackReply, RangeCallback] {

        override def unicast: (Observer[RangeCallbackReply], Observable[RangeCallback]) = {
          (proxy.input, proxy.output)
        }
      }
    }

  val putPipe: IO[Pipe[PutCallbackReply, PutCallback]] = {
    for {
      websocket ← connection
      proxy = GrpcProxyClient.proxy(service, "put", websocket, generatedMessageCodec, protobufDynamicCodec(PutCallback))
    } yield {
      new Pipe[PutCallbackReply, PutCallback] {
        override def unicast: (Observer[PutCallbackReply], Observable[PutCallback]) = {
          (proxy.input, proxy.output)
        }
      }
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
  ): IO[Option[Array[Byte]]] = getPipe.flatMap(ClientGet(datasetId, version, searchCallbacks).runStream)

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
    Observable.fromIO(rangePipe).flatMap(ClientRange(datasetId, version, searchCallbacks).runStream)

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
  ): IO[Option[Array[Byte]]] = putPipe.flatMap(ClientPut(datasetId, version, putCallbacks, encryptedValue).runStream)

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
