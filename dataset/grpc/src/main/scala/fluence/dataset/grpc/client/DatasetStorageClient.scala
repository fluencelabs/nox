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
import fluence.stream.Connection
import monix.execution.Scheduler
import monix.reactive.{MulticastStrategy, Observable, Observer, Pipe}

import scala.language.higherKinds

/**
 * Client for interaction with the database.
 *
 */
class DatasetStorageClient[F[_]: Effect](connection: Connection)(
  implicit sch: Scheduler
) extends DatasetStorageRpc[F, Observable] with slogging.LazyLogging {

  import fluence.transport.ProtobufCodec._

  private val service = "fluence.dataset.protobuf.grpc.DatasetStorageRpc"

  def handleGet(requests: Observable[GetCallbackReply]): IO[Observable[GetCallback]] = {

    val mapped = requests.mapEval[IO, Array[Byte]](ab ⇒ generatedMessageCodec.runF[IO](ab))
    for {
      responseObservable ← connection.handle(service, "get", mapped)
      responseDeserialized = responseObservable.mapEval[IO, GetCallback](
        resp ⇒ protobufDynamicCodec(GetCallback).runF[IO](resp)
      )
    } yield {
      responseDeserialized
    }
  }

  def handleRange(requests: Observable[RangeCallbackReply]): IO[Observable[RangeCallback]] = {
    val mapped = requests.mapEval[IO, Array[Byte]](ab ⇒ generatedMessageCodec.runF[IO](ab))
    for {
      responseObservable ← connection.handle(service, "range", mapped)
      responseDeserialized = responseObservable.mapEval[IO, RangeCallback](
        resp ⇒ protobufDynamicCodec(RangeCallback).runF[IO](resp)
      )
    } yield {
      responseDeserialized
    }
  }

  def handlePut(requests: Observable[PutCallbackReply]): IO[Observable[PutCallback]] = {

    val mapped = requests.mapEval[IO, Array[Byte]](ab ⇒ generatedMessageCodec.runF[IO](ab))
    for {
      responseObservable ← connection.handle(service, "put", mapped)
      responseDeserialized = responseObservable.mapEval[IO, PutCallback](
        resp ⇒ protobufDynamicCodec(PutCallback).runF[IO](resp)
      )
    } yield {
      responseDeserialized
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
  ): IO[Option[Array[Byte]]] = ClientGet(datasetId, version, searchCallbacks).runStream(handleGet)

  /**
   * Initiates ''Range'' operation in remote MerkleBTree.
   *
   * @param datasetId       Dataset ID
   * @param version         Dataset version expected to the client
   * @param searchCallbacks Wrapper for all callback needed for ''Range'' operation to the BTree
   * @return returns stream of found value.
   */
  override def range(
    datasetId: Array[Byte],
    version: Long,
    searchCallbacks: BTreeRpc.SearchCallback[F]
  ): Observable[(Array[Byte], Array[Byte])] =
    Observable.fromIO(ClientRange(datasetId, version, searchCallbacks).runStream(handleRange)).flatten

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
  ): IO[Option[Array[Byte]]] = ClientPut(datasetId, version, putCallbacks, encryptedValue).runStream(handlePut)

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

object DatasetStorageClient {
  def apply[F[_] : Effect](streamHandler: Connection)(
    implicit sch: Scheduler
  ): DatasetStorageRpc[F, Observable] = new DatasetStorageClient(streamHandler)
}
