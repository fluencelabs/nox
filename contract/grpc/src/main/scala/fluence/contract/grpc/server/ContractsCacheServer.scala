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
import cats.effect.IO
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
 * @tparam C Domain-level Contract
 */
class ContractsCacheServer[C](cache: ContractsCacheRpc[C])(
  implicit
  codec: Codec[IO, C, BasicContract],
  keyK: Kleisli[IO, Array[Byte], Key]
) extends ContractsCacheGrpc.ContractsCache {

  override def find(request: FindRequest): Future[BasicContract] =
    (
      for {
        k ← keyK(request.id.toByteArray)
        resp ← cache.find(k).flatMap[BasicContract] {
          case Some(c) ⇒ codec.encode(c)
          case None ⇒ IO.raiseError(new NoSuchElementException(""))
        }
      } yield resp
    ).unsafeToFuture()

  override def cache(request: BasicContract): Future[CacheResponse] =
    (
      for {
        c ← codec.decode(request)
        resp ← cache.cache(c)
      } yield CacheResponse(resp)
    ).unsafeToFuture()
}
