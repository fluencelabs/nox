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

import cats.data.Kleisli
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.{ MonadError, ~> }
import fluence.codec.Codec
import fluence.contract.protocol.ContractsCacheRpc
import fluence.contract.grpc._
import fluence.kad.protocol.Key

import scala.concurrent.Future
import scala.language.higherKinds

/**
 * ContractsCache GRPC server implementation.
 *
 * @param cache Delegate implementation
 * @param F MonadError instance
 * @param run Runs F and produces Future
 * @tparam F Effect
 * @tparam C Domain-level Contract
 */
class ContractsCacheServer[F[_], C](cache: ContractsCacheRpc[F, C])(
    implicit
    F: MonadError[F, Throwable],
    codec: Codec[F, C, BasicContract],
    keyK: Kleisli[F, Array[Byte], Key],
    run: F ~> Future
) extends ContractsCacheGrpc.ContractsCache {

  override def find(request: FindRequest): Future[BasicContract] =
    run(
      for {
        k ← keyK(request.id.toByteArray)
        resp ← cache.find(k).flatMap[BasicContract] {
          case Some(c) ⇒ codec.encode(c)
          case None    ⇒ F.raiseError(new NoSuchElementException(""))
        }
      } yield resp
    )

  override def cache(request: BasicContract): Future[CacheResponse] =
    run(
      for {
        c ← codec.decode(request)
        resp ← cache.cache(c)
      } yield CacheResponse(resp)
    )
}
