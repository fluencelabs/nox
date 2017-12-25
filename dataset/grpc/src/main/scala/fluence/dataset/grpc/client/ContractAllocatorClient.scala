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

import cats.syntax.functor._
import cats.{ Functor, ~> }
import fluence.dataset.grpc.Contract
import fluence.dataset.grpc.ContractAllocatorGrpc.ContractAllocatorStub
import fluence.dataset.protocol.ContractAllocatorRpc

import scala.concurrent.Future
import scala.language.higherKinds

class ContractAllocatorClient[F[_] : Functor, C](
    stub: ContractAllocatorStub,
    serialize: C ⇒ Contract,
    deserialize: Contract ⇒ C
)(implicit run: Future ~> F)
  extends ContractAllocatorRpc[F, C] {

  /**
   * Offer a contract. Node should check and preallocate required resources, save offer, and sign it.
   *
   * @param contract A blank contract
   * @return Signed contract, or F is an error
   */
  override def offer(contract: C): F[C] =
    run(stub.offer(serialize(contract))).map(deserialize)

  /**
   * Allocate dataset: store the contract, create storage structures, form cluster.
   *
   * @param contract A sealed contract with all nodes and client signatures
   * @return Allocated contract
   */
  override def allocate(contract: C): F[C] =
    run(stub.allocate(serialize(contract))).map(deserialize)
}
