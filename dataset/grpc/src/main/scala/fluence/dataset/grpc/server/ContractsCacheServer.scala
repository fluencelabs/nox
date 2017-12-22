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

import cats.{ MonadError, ~> }
import fluence.dataset.grpc.{ CacheResponse, Contract, ContractsCacheGrpc, FindRequest }
import fluence.dataset.protocol.ContractsCacheRpc
import fluence.kad.protocol.Key
import cats.syntax.flatMap._
import cats.syntax.functor._
import scala.concurrent.Future
import scala.language.higherKinds

class ContractsCacheServer[F[_], C](
    cache: ContractsCacheRpc[F, C],
    serialize: C ⇒ Contract,
    deserialize: Contract ⇒ C

)(implicit F: MonadError[F, Throwable], run: F ~> Future)
  extends ContractsCacheGrpc.ContractsCache {
  override def find(request: FindRequest): Future[Contract] =
    run(cache.find(Key(request.id.asReadOnlyByteBuffer())).flatMap[Contract] {
      case Some(c) ⇒ F.pure(serialize(c))
      case None    ⇒ F.raiseError(new NoSuchElementException(""))
    })

  override def cache(request: Contract): Future[CacheResponse] =
    run(cache.cache(deserialize(request)).map(CacheResponse(_)))
}
