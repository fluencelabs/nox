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
import cats.{ Monad, ~> }
import com.google.protobuf.ByteString
import fluence.dataset.grpc
import fluence.dataset.grpc.{ Contract, FindRequest }
import fluence.dataset.protocol.ContractsAllocatorApi
import fluence.kad.protocol.Key
import io.grpc.stub.StreamObserver

import scala.concurrent.{ Future, Promise }
import scala.language.higherKinds

/**
 * User-facing ContractsAllocatorApi client.
 *
 * @param stub        GRPC stub for DatasetContracts API
 * @param serialize   Serialize domain-level Contract to Grpc
 * @param deserialize Deserialize from Grpc to domain-level Contract
 * @param run         Run scala future to get F
 * @tparam F Effect
 * @tparam C Domain-level Contract type
 */
class ContractsAllocatorApiClient[F[_] : Monad, C](
    stub: grpc.DatasetContractsApiGrpc.DatasetContractsApiStub,
    serialize: C ⇒ Contract,
    deserialize: Contract ⇒ C)(implicit run: Future ~> F)
  extends ContractsAllocatorApi[F, C] {

  /**
   * According with contract, offers contract to participants, then seals the list of agreements on client side
   * and performs allocation. In case of any error, result is a failure
   *
   * @param contract         Contract to allocate
   * @param sealParticipants Client's callback to seal list of participants with a signature
   * @return Sealed contract with a list of participants, or failure
   */
  override def allocate(contract: C, sealParticipants: C ⇒ F[C]): F[C] = {
    val withParticipants = Promise[Contract]()
    val finalized = Promise[Contract]()
    val waitingFinalized = new AtomicBoolean(false)

    val str = stub.allocate(new StreamObserver[Contract] {
      override def onError(t: Throwable): Unit =
        if (!waitingFinalized.getAndSet(true))
          withParticipants.failure(t)
        else
          finalized.failure(t)

      override def onCompleted(): Unit = ()

      override def onNext(value: Contract): Unit =
        if (!waitingFinalized.getAndSet(true))
          withParticipants.success(value)
        else
          finalized.success(value)
    })

    str.onNext(serialize(contract))

    for {
      fullContract ← run(withParticipants.future)
      sealedContract ← sealParticipants(deserialize(fullContract))
      _ = str.onNext(serialize(sealedContract))
      finalizedContract ← run(finalized.future)
    } yield deserialize(finalizedContract)
  }

  /**
   * Tries to find a contract by its Kademlia key, or fails.
   *
   * @param key Dataset ID
   * @return Found contract, or failure
   */
  override def find(key: Key): F[C] =
    run(stub.find(FindRequest(ByteString.copyFrom(key.origin)))).map(deserialize)
}
