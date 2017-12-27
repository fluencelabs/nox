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

import cats.{ Monad, ~> }
import cats.syntax.functor._
import cats.syntax.flatMap._
import fluence.codec.Codec
import fluence.dataset.grpc.{ Contract, ContractAllocatorGrpc }
import fluence.dataset.protocol.ContractAllocatorRpc

import scala.concurrent.Future
import scala.language.higherKinds

class ContractAllocatorServer[F[_], C](contractAllocator: ContractAllocatorRpc[F, C])(implicit
    F: Monad[F],
    codec: Codec[F, C, Contract],
    run: F ~> Future)
  extends ContractAllocatorGrpc.ContractAllocator {

  override def offer(request: Contract): Future[Contract] =
    run(
      for {
        c ← codec.decode(request)
        offered ← contractAllocator.offer(c)
        resp ← codec.encode(offered)
      } yield resp
    )

  override def allocate(request: Contract): Future[Contract] =
    run(
      for {
        c ← codec.decode(request)
        allocated ← contractAllocator.allocate(c)
        resp ← codec.encode(allocated)
      } yield resp
    )
}
