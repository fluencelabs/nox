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

import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.{ Monad, ~> }
import fluence.codec.Codec
import fluence.dataset.grpc.BasicContract
import fluence.dataset.grpc.ContractAllocatorGrpc.ContractAllocatorStub
import fluence.dataset.protocol.ContractAllocatorRpc

import scala.concurrent.Future
import scala.language.higherKinds

class ContractAllocatorClient[F[_] : Monad, C](
    stub: ContractAllocatorStub
)(implicit codec: Codec[F, C, BasicContract], run: Future ~> F)
  extends ContractAllocatorRpc[F, C] {

  /**
   * Offer a contract. Node should check and preallocate required resources, save offer, and sign it.
   *
   * @param contract A blank contract
   * @return Signed contract, or F is an error
   */
  override def offer(contract: C): F[C] =
    for {
      offer ← codec.encode(contract)
      resp ← run(stub.offer(offer))
      contract ← codec.decode(resp)
    } yield contract

  /**
   * Allocate dataset: store the contract, create storage structures, form cluster.
   *
   * @param contract A sealed contract with all nodes and client signatures
   * @return Allocated contract
   */
  override def allocate(contract: C): F[C] =
    for {
      offer ← codec.encode(contract)
      resp ← run(stub.allocate(offer))
      contract ← codec.decode(resp)
    } yield contract

}
