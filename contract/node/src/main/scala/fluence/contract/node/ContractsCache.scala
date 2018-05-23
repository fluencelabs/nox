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

import cats.Monad
import cats.data.EitherT
import cats.effect.IO
import fluence.contract.node.cache.ContractRecord
import fluence.contract.ops.ContractRead
import fluence.contract.protocol.ContractsCacheRpc
import fluence.crypto.CryptoError
import fluence.crypto.signature.SignAlgo.CheckerFn
import fluence.kad.protocol.Key
import fluence.kvstore.{ReadWriteKVStore, StoreError}

import scala.concurrent.duration.FiniteDuration
import scala.language.{higherKinds, implicitConversions}

/**
 * Contracts cache.
 *
 * @param nodeId Current node id, to check participation
 * @param storage Contracts storage
 * @param cacheTtl Cache time-to-live
 * @tparam F Effect
 * @tparam C Contract
 */
class ContractsCache[F[_]: Monad, C: ContractRead](
  nodeId: Key,
  storage: ReadWriteKVStore[Key, ContractRecord[C]],
  cacheTtl: FiniteDuration,
  clock: Clock
)(implicit checkerFn: CheckerFn)
    extends ContractsCacheRpc[C] with slogging.LazyLogging {

  import ContractRead._

  private lazy val ttlMillis = cacheTtl.toMillis

  private lazy val cacheEnabled = ttlMillis > 0

  private def isExpired(cr: ContractRecord[C]): Boolean =
    !cr.contract.participants.contains(nodeId) &&
      java.time.Duration.between(cr.lastUpdated, clock.instant()).toMillis >= ttlMillis

  private def canBeCached(contract: C): EitherT[IO, CryptoError, Boolean] =
    if (cacheEnabled && !contract.participants.contains(nodeId))
      contract.isActiveContract()
    else
      EitherT.rightT(false)

  /**
   * Find a contract in local storage.
   *
   * @param id Dataset ID
   * @return Optional locally found contract
   */
  override def find(id: Key): IO[Option[C]] =
    storage
      .get(id)
      .run[IO]
      .flatMap {
        case Some(contractRecord) if isExpired(contractRecord) ⇒
          storage
            .remove(id)
            .run[IO]
            .map(_ ⇒ Option.empty[C])

        case Some(contractRecord) ⇒
          contractRecord.contract
            .isBlankOffer[IO]
            .leftMap(StoreError(_))
            .map {
              case false ⇒ Some(contractRecord.contract)
              case true ⇒ Option.empty[C]
            }

        case None ⇒
          EitherT.rightT[IO, StoreError](Option.empty[C])
      }
      .recover {
        case error ⇒
          logger.warn(s"Contract with $id wasn't found in cache, cause: $error", error)
          Option.empty[C]
      }
      .toIO

  /**
   * Ask to add contract to local storage.
   *
   * @param contract Contract to cache
   * @return If the contract is cached or not
   */
  override def cache(contract: C): IO[Boolean] =
    canBeCached(contract)
      .leftMap(StoreError(_))
      .flatMap {
        case false ⇒
          EitherT.rightT[IO, StoreError](false)
        case true ⇒
          // We're deciding to cache basing on crypto check, done with canBeCached, and (signed) version number only
          // It allows us to avoid multiplexing network calls with asking to cache stale contracts
          storage.get(contract.id).run[IO].flatMap {
            case Some(cr) if cr.contract.version < contract.version ⇒ // Contract updated
              storage
                .put(contract.id, ContractRecord(contract, clock.instant()))
                .run[IO]
                .map(_ ⇒ true)

            case Some(_) ⇒ // Can't update contract with an old version
              EitherT.rightT[IO, StoreError](false)

            case None ⇒ // Contract is unknown, save it
              storage
                .put(contract.id, ContractRecord(contract, clock.instant()))
                .run[IO]
                .map(_ ⇒ true)
          }
      }
      .recover {
        case error ⇒
          logger.warn(s"Contract ${contract.id} can't be cached, cause: $error", error)
          false
      }
      .toIO

  // todo will be removed when all API will be 'EitherT compatible'
  private implicit class EitherT2IO[E <: Throwable, V](origin: EitherT[IO, E, V]) {

    def toIO: IO[V] =
      origin.value.flatMap {
        case Right(value) ⇒ IO.pure(value)
        case Left(error) ⇒ IO.raiseError(error)
      }
  }

}
