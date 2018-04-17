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

package fluence.contract.grpc.server

import cats.effect.IO
import fluence.codec.Codec
import fluence.contract.protocol.ContractAllocatorRpc
import fluence.contract.ops.ContractValidate
import fluence.crypto.SignAlgo.CheckerFn
import fluence.contract.protobuf.BasicContract
import fluence.contract.protobuf.grpc.ContractAllocatorGrpc

import scala.concurrent.Future

// todo unit test
class ContractAllocatorServer[C: ContractValidate](contractAllocator: ContractAllocatorRpc[C])(
  implicit
  codec: Codec[IO, C, BasicContract],
  checkerFn: CheckerFn,
) extends ContractAllocatorGrpc.ContractAllocator {
  import ContractValidate.ContractValidatorOps

  override def offer(request: BasicContract): Future[BasicContract] =
    (
      for {
        contract ← codec.decode(request)
        // contract from the outside required validation
        _ ← contract.validateME[IO]
        offered ← contractAllocator.offer(contract)
        // we should validate contract before send outside for 'offering'
        _ ← offered.validateME[IO]
        resp ← codec.encode(offered)
      } yield resp
    ).unsafeToFuture()

  override def allocate(request: BasicContract): Future[BasicContract] =
    (
      for {
        contract ← codec.decode(request)
        // contract from the outside required validation
        _ ← contract.validateME[IO]
        allocated ← contractAllocator.allocate(contract)
        // we should validate contract before send outside for 'allocating'
        _ ← allocated.validateME[IO]
        resp ← codec.encode(allocated)
      } yield resp
    ).unsafeToFuture()
}
