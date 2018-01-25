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

import java.util.concurrent.atomic.AtomicBoolean

import cats.data.Kleisli
import cats.{ MonadError, ~> }
import cats.syntax.functor._
import cats.syntax.flatMap._
import fluence.codec.Codec
import fluence.dataset.grpc.{ BasicContract, DatasetContractsApiGrpc, FindRequest }
import fluence.dataset.protocol.ContractsApi
import fluence.kad.protocol.Key
import io.grpc.stub.StreamObserver

import scala.concurrent.{ Future, Promise }
import scala.language.higherKinds

class ContractsApiServer[F[_], C](
    api: ContractsApi[F, C]
)(implicit
    F: MonadError[F, Throwable],
    codec: Codec[F, C, BasicContract],
    keyK: Kleisli[F, Array[Byte], Key],
    run: F ~> Future,
    hold: Future ~> F)
  extends DatasetContractsApiGrpc.DatasetContractsApi {

  override def allocate(responseObserver: StreamObserver[BasicContract]): StreamObserver[BasicContract] =
    new StreamObserver[BasicContract] {
      private val initialReceived = new AtomicBoolean(false)
      private val callbackPromise = Promise[BasicContract]()

      override def onError(t: Throwable): Unit =
        if (initialReceived.get()) responseObserver.onError(t)
        else callbackPromise.tryFailure(t)

      override def onCompleted(): Unit =
        callbackPromise.tryFailure(new IllegalArgumentException("Stream completed before callback was processed"))

      override def onNext(value: BasicContract): Unit =
        if (!initialReceived.getAndSet(true))
          run(
            for {
              c ← codec.decode(value)
              allocated ← api.allocate(
                c,
                signed ⇒
                  for {
                    binSigned ← codec.encode(signed)
                    _ = println("signed: " + signed)
                    _ = println("bin signed " + new String(java.util.Base64.getEncoder.encode(binSigned.id.toByteArray)))
                    _ ← F.catchNonFatal(responseObserver.onNext(binSigned))
                    clientSealed ← hold(callbackPromise.future)
                    _ = println("client sealed " + clientSealed)
                    clientRaw ← codec.decode(clientSealed)
                  } yield clientRaw
              )
              resp ← codec.encode(allocated)
              _ ← F.catchNonFatal(responseObserver.onNext(resp))
              _ ← F.catchNonFatal(responseObserver.onCompleted())
            } yield ()
          )
        else callbackPromise.trySuccess(value)
    }

  override def find(request: FindRequest): Future[BasicContract] =
    run(
      for {
        k ← keyK(request.id.toByteArray)
        contract ← api.find(k)
        resp ← codec.encode(contract)
      } yield resp
    )
}
