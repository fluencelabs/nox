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

package fluence.contract.node

import java.time.Clock

import cats.{~>, Monad}
import cats.effect.IO
import cats.syntax.applicative._
import cats.syntax.functor._
import fluence.contract.node.cache.ContractRecord
import fluence.contract.ops.ContractRead
import fluence.contract.protocol.ContractsCacheRpc
import fluence.crypto.SignAlgo.CheckerFn
import fluence.kad.protocol.Key
import fluence.storage.KVStore

import scala.concurrent.duration.FiniteDuration
import scala.language.{higherKinds, implicitConversions}

/**
 * Contracts cache.
 * TODO: we have a number of toIO convertions due to wrong [[KVStore.get]] signature; it should be fixed
 *
 * @param nodeId Current node id, to check participation
 * @param storage Contracts storage
 * @param cacheTtl Cache time-to-live
 * @tparam F Effect
 * @tparam C Contract
 */
class ContractsCache[F[_]: Monad, C: ContractRead](
  nodeId: Key,
  storage: KVStore[F, Key, ContractRecord[C]],
  cacheTtl: FiniteDuration,
  clock: Clock,
  toIO: F ~> IO
)(implicit checkerFn: CheckerFn)
    extends ContractsCacheRpc[C] {

  import ContractRead._

  private lazy val ttlMillis = cacheTtl.toMillis

  private lazy val cacheEnabled = ttlMillis > 0

  // TODO: remove Instant.now() usage
  private def isExpired(cr: ContractRecord[C]): Boolean =
    !cr.contract.participants.contains(nodeId) &&
      java.time.Duration.between(cr.lastUpdated, clock.instant()).toMillis >= ttlMillis

  private def canBeCached(contract: C): F[Boolean] =
    if (cacheEnabled && !contract.participants.contains(nodeId))
      contract.isActiveContract().value.map(_.contains(true))
    else false.pure[F]

  /**
   * Find a contract in local storage.
   *
   * @param id Dataset ID
   * @return Optional locally found contract
   */
  override def find(id: Key): IO[Option[C]] =
    toIO(storage.get(id)).flatMap {
      case Some(cr) if isExpired(cr) ⇒
        toIO(
          storage
            .remove(id)
            .map(_ ⇒ None)
        )

      case Some(cr) ⇒
        for {
          ibo ← cr.contract.isBlankOffer[IO]().value
        } yield Option(cr.contract).filter(_ ⇒ ibo.contains(false))

      case None ⇒
        Option.empty[C].pure[IO]
    }

  /**
   * Ask to add contract to local storage.
   *
   * @param contract Contract to cache
   * @return If the contract is cached or not
   */
  override def cache(contract: C): IO[Boolean] =
    toIO(canBeCached(contract)).flatMap {
      case false ⇒ false.pure[IO]
      case true ⇒
        // We're deciding to cache basing on crypto check, done with canBeCached, and (signed) version number only
        // It allows us to avoid multiplexing network calls with asking to cache stale contracts
        toIO(storage.get(contract.id)).flatMap {
          case Some(cr) if cr.contract.version < contract.version ⇒ // Contract updated
            toIO(
              storage
                .put(contract.id, ContractRecord(contract, clock.instant()))
                .map(_ ⇒ true)
            )

          case Some(_) ⇒ // Can't update contract with an old version
            false.pure[IO]

          case None ⇒ // Contract is unknown, save it
            toIO(
              storage
                .put(contract.id, ContractRecord(contract, clock.instant()))
                .map(_ ⇒ true)
            )
        }
    }
}
