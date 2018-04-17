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

package fluence.dataset.grpc.node

import cats.effect.Async
import cats.{~>, Monad}
import fluence.dataset.node.{NodeGet, NodePut, NodeRange}
import fluence.dataset.protocol.DatasetStorageRpc
import fluence.protobuf.dataset._
import fluence.protobuf.dataset.grpc.DatasetStorageRpcGrpc
import io.grpc.stub.StreamObserver
import monix.eval.Task
import monix.execution.Scheduler
import monix.reactive.{Observable, Observer}

import scala.language.higherKinds

/**
 * Server implementation of [[DatasetStorageRpcGrpc.DatasetStorageRpc]], allows talking to client via network.
 * All public methods called from the server side.
 * DatasetStorageServer is active and initiates requests to client.
 *
 * @param service Server implementation of [[DatasetStorageRpc]] to which the calls will be delegated
 * @tparam F A box for returning value
 */
class DatasetStorageServer[F[_]: Async](
  service: DatasetStorageRpc[F, Observable]
)(
  implicit
  F: Monad[F],
  runF: F ~> Task,
  scheduler: Scheduler
) extends DatasetStorageRpcGrpc.DatasetStorageRpc with slogging.LazyLogging {

  import fluence.dataset.grpc.GrpcMonix._

  override def get(responseObserver: StreamObserver[GetCallback]): StreamObserver[GetCallbackReply] = {

    val resp: Observer[GetCallback] = responseObserver
    val (repl, stream) = streamObservable[GetCallbackReply]

    NodeGet(service).runStream(resp, repl)

    stream
  }

  override def range(responseObserver: StreamObserver[RangeCallback]): StreamObserver[RangeCallbackReply] = {

    val resp: Observer[RangeCallback] = responseObserver
    val (repl, stream) = streamObservable[RangeCallbackReply]

    NodeRange(service, resp, repl).runStream(resp, repl)

    stream
  }

  override def put(responseObserver: StreamObserver[PutCallback]): StreamObserver[PutCallbackReply] = {
    val resp: Observer[PutCallback] = responseObserver
    val (repl, stream) = streamObservable[PutCallbackReply]

    NodePut(service, resp, repl).runStream(resp, repl)

    stream
  }
}
