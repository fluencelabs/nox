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

import cats.syntax.applicativeError._
import cats.syntax.functor._
import cats.{ MonadError, ~> }
import com.google.protobuf.ByteString
import fluence.dataset.grpc.ContractsCacheGrpc.ContractsCacheStub
import fluence.dataset.grpc.{ Contract, FindRequest }
import fluence.dataset.protocol.ContractsCacheRpc
import fluence.kad.protocol.Key

import scala.concurrent.Future
import scala.language.higherKinds

class ContractsCacheClient[F[_], C](
    stub: ContractsCacheStub,
    serialize: C ⇒ Contract,
    deserialize: Contract ⇒ C)(implicit run: Future ~> F, F: MonadError[F, Throwable])
  extends ContractsCacheRpc[F, C] {

  /**
   * Tries to find a contract in local cache.
   *
   * @param id Dataset ID
   * @return Optional locally found contract
   */
  override def find(id: Key): F[Option[C]] =
    run(
      stub.find(FindRequest(ByteString.copyFrom(id.origin)))
    ).map(c ⇒ Option(deserialize(c))).recover {
        case _ ⇒ None
      }

  /**
   * Ask node to cache the contract.
   *
   * @param contract Contract to cache
   * @return If the contract is cached or not
   */
  override def cache(contract: C): F[Boolean] =
    run(
      stub.cache(serialize(contract))
    ).map(_.cached)
}
