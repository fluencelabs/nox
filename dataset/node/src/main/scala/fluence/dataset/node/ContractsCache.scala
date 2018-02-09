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

package fluence.dataset.node

import java.time.Instant

import cats.MonadError
import cats.syntax.applicative._
import cats.syntax.applicativeError._
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.crypto.signature.SignatureChecker
import fluence.dataset.contract.ContractRead
import fluence.dataset.node.contract.ContractRecord
import fluence.dataset.protocol.ContractsCacheRpc
import fluence.kad.protocol.Key
import fluence.storage.KVStore

import scala.concurrent.duration.FiniteDuration
import scala.language.{ higherKinds, implicitConversions }

/**
 * Contracts cache.
 *
 * @param nodeId Current node id, to check participation
 * @param storage     Contracts storage
 * @param checker Signature checker
 * @param cacheTtl    Cache time-to-live
 * @param ME          Monad error
 * @tparam F Effect
 * @tparam C Contract
 */
class ContractsCache[F[_], C : ContractRead](
    nodeId: Key,
    storage: KVStore[F, Key, ContractRecord[C]],
    checker: SignatureChecker[F],
    cacheTtl: FiniteDuration)(implicit ME: MonadError[F, Throwable]) extends ContractsCacheRpc[F, C] {

  import ContractRead._

  private lazy val ttlMillis = cacheTtl.toMillis

  private lazy val cacheEnabled = ttlMillis > 0

  // TODO: remove Instant.now() usage
  private def isExpired(cr: ContractRecord[C]): Boolean =
    !cr.contract.participants.contains(nodeId) &&
      java.time.Duration.between(cr.lastUpdated, Instant.now()).toMillis >= ttlMillis

  private def canBeCached(contract: C): F[Boolean] = {
    if (cacheEnabled && !contract.participants.contains(nodeId))
      contract.isActiveContract(checker)
    else false.pure[F]
  }

  /**
   * Find a contract in local storage.
   *
   * @param id Dataset ID
   * @return Optional locally found contract
   */
  override def find(id: Key): F[Option[C]] = {
    storage.get(id).attempt.map(_.toOption).flatMap {
      case Some(cr) if isExpired(cr) ⇒
        storage
          .remove(id)
          .map(_ ⇒ None)

      case Some(cr) ⇒
        cr.contract.isBlankOffer(checker).map { b ⇒
          if (!b) Some(cr.contract) else None
        }
      case None ⇒ Option.empty[C].pure[F]
    }
  }

  /**
   * Ask to add contract to local storage.
   *
   * @param contract Contract to cache
   * @return If the contract is cached or not
   */
  override def cache(contract: C): F[Boolean] = {
    canBeCached(contract).flatMap {
      case false ⇒ false.pure[F]
      case true ⇒
        // We're deciding to cache basing on crypto check, done with canBeCached, and (signed) version number only
        // It allows us to avoid multiplexing network calls with asking to cache stale contracts
        storage.get(contract.id).attempt.map(_.toOption).flatMap {
          case Some(cr) if cr.contract.version < contract.version ⇒ // Contract updated
            storage
              .put(contract.id, ContractRecord(contract))
              .map(_ ⇒ true)

          case Some(_) ⇒ // Can't update contract with an old version
            false.pure[F]

          case None ⇒ // Contract is unknown, save it
            storage
              .put(contract.id, ContractRecord(contract))
              .map(_ ⇒ true)
        }
    }
  }
}
