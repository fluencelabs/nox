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

package fluence.contract.grpc.client

import cats.effect.IO
import fluence.codec.Codec
import fluence.contract.protocol.ContractAllocatorRpc
import fluence.contract.grpc.{BasicContract, ContractAllocatorGrpc}
import fluence.contract.grpc.ContractAllocatorGrpc.ContractAllocatorStub
import io.grpc.{CallOptions, ManagedChannel}

import scala.concurrent.{ExecutionContext, Future}

class ContractAllocatorClient[C](
  stub: ContractAllocatorStub
)(
  implicit
  codec: Codec[IO, C, BasicContract],
  ec: ExecutionContext
) extends ContractAllocatorRpc[C] {

  private def run[A](fa: Future[A]): IO[A] = IO.fromFuture(IO(fa))

  /**
   * Offer a contract. Node should check and preallocate required resources, save offer, and sign it.
   *
   * @param contract A blank contract
   * @return Signed contract, or F is an error
   */
  override def offer(contract: C): IO[C] =
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
  override def allocate(contract: C): IO[C] =
    for {
      offer ← codec.encode(contract)
      resp ← run(stub.allocate(offer))
      contract ← codec.decode(resp)
    } yield contract

}

object ContractAllocatorClient {

  /**
   * Shorthand to register inside NetworkClient.
   *
   * @param channel     Channel to remote node
   * @param callOptions Call options
   */
  def register[C]()(
    channel: ManagedChannel,
    callOptions: CallOptions
  )(
    implicit
    codec: Codec[IO, C, BasicContract],
    ec: ExecutionContext
  ): ContractAllocatorRpc[C] =
    new ContractAllocatorClient[C](new ContractAllocatorGrpc.ContractAllocatorStub(channel, callOptions))

}
