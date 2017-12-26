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

package fluence.dataset.grpc.server

import java.nio.ByteBuffer

import cats.{ Monad, ~> }
import cats.syntax.functor._
import cats.syntax.flatMap._
import fluence.codec.Codec
import fluence.dataset.grpc.{ Contract, DatasetContractsApiGrpc, FindRequest }
import fluence.dataset.protocol.ContractsAllocatorApi
import fluence.kad.protocol.Key
import io.grpc.stub.StreamObserver

import scala.concurrent.Future
import scala.language.higherKinds

class ContractsAllocatorApiServer[F[_], C](
    api: ContractsAllocatorApi[F, C]
)(implicit
    F: Monad[F],
    codec: Codec[F, C, Contract],
    keyCodec: Codec[F, Key, ByteBuffer],
    run: F ~> Future)
  extends DatasetContractsApiGrpc.DatasetContractsApi {

  // TODO: implement
  override def allocate(responseObserver: StreamObserver[Contract]): StreamObserver[Contract] =
    new StreamObserver[Contract] {
      override def onError(t: Throwable): Unit = ???

      override def onCompleted(): Unit = ???

      override def onNext(value: Contract): Unit = ???
    }

  override def find(request: FindRequest): Future[Contract] =
    run(
      for {
        k ← keyCodec.decode(request.id.asReadOnlyByteBuffer())
        contract ← api.find(k)
        resp ← codec.encode(contract)
      } yield resp
    )
}
