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

import java.util.concurrent.atomic.AtomicBoolean

import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.{ MonadError, ~> }
import com.google.protobuf.ByteString
import fluence.codec.Codec
import fluence.dataset.grpc
import fluence.dataset.grpc.{ BasicContract, FindRequest }
import fluence.dataset.protocol.ContractsApi
import fluence.kad.protocol.Key
import io.grpc.stub.StreamObserver
import io.grpc.{ CallOptions, ManagedChannel }
import fluence.transport.grpc.GrpcCodecs._

import scala.concurrent.{ Future, Promise }
import scala.language.higherKinds

/**
 * User-facing ContractsAllocatorApi client.
 *
 * @param stub GRPC stub for DatasetContracts API
 * @param run  Run scala future to get F
 * @tparam F Effect
 * @tparam C Domain-level Contract type
 */
class ContractsApiClient[F[_], C](
    stub: grpc.DatasetContractsApiGrpc.DatasetContractsApiStub)(implicit
    F: MonadError[F, Throwable],
    codec: Codec[F, C, BasicContract],
    run: Future ~> F)
  extends ContractsApi[F, C] {

  private val keyBS = Codec.codec[F, ByteString, Key].inverse

  /**
   * According with contract, offers contract to participants, then seals the list of agreements on client side
   * and performs allocation. In case of any error, result is a failure
   *
   * @param contract         Contract to allocate
   * @param sealParticipants Client's callback to seal list of participants with a signature
   * @return Sealed contract with a list of participants, or failure
   */
  override def allocate(contract: C, sealParticipants: C ⇒ F[C]): F[C] = {
    val withParticipants = Promise[BasicContract]()
    val finalized = Promise[BasicContract]()
    val waitingFinalized = new AtomicBoolean(false)

    val str = stub.allocate(new StreamObserver[BasicContract] {
      override def onError(t: Throwable): Unit =
        if (!waitingFinalized.getAndSet(true))
          withParticipants.failure(t)
        else
          finalized.failure(t)

      override def onCompleted(): Unit = ()

      override def onNext(value: BasicContract): Unit =
        if (!waitingFinalized.getAndSet(true))
          withParticipants.success(value)
        else
          finalized.success(value)
    })

    for {
      c ← codec.encode(contract)
      _ = str.onNext(c)
      fullContract ← run(withParticipants.future)
      fcRaw ← codec.decode(fullContract)

      sealedContract ← sealParticipants(fcRaw)
      sealedBin ← codec.encode(sealedContract)

      _ = str.onNext(sealedBin)
      finalizedContract ← run(finalized.future)
      finalizedRaw ← codec.decode(finalizedContract)

      _ ← F.catchNonFatal(str.onCompleted())
    } yield finalizedRaw
  }

  /**
   * Tries to find a contract by its Kademlia key, or fails.
   *
   * @param key Dataset ID
   * @return Found contract, or failure
   */
  override def find(key: Key): F[C] =
    for {
      k ← keyBS(key)
      resp ← run(stub.find(FindRequest(k)))
      raw ← codec.decode(resp)
    } yield raw
}

object ContractsApiClient {
  /**
   * Shorthand to register inside NetworkClient.
   *
   * @param channel     Channel to remote node
   * @param callOptions Call options
   */
  def register[F[_], C]()(channel: ManagedChannel, callOptions: CallOptions)(implicit
    F: MonadError[F, Throwable],
    codec: Codec[F, C, BasicContract],
    run: Future ~> F): ContractsApi[F, C] =
    new ContractsApiClient[F, C](new grpc.DatasetContractsApiGrpc.DatasetContractsApiStub(channel, callOptions))

}
