package fluence.dataset.allocate

import java.time.Instant

import cats.MonadError
import cats.syntax.applicative._
import cats.syntax.applicativeError._
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.kad.Key
import fluence.node.storage.KVStore

import scala.concurrent.duration.FiniteDuration
import scala.language.{ higherKinds, implicitConversions }

/**
 * Contracts cache
 *
 * @param storage     Contracts storage
 * @param contractOps Contract ops
 * @param cacheTtl    Cache time-to-live
 * @param ME          Monad error
 * @tparam F Effect
 * @tparam C Contract
 */
class ContractsCache[F[_], C](
    storage: KVStore[F, Key, ContractRecord[C]],
    contractOps: C ⇒ ContractOps[C],
    cacheTtl: FiniteDuration)(implicit ME: MonadError[F, Throwable]) extends ContractsCacheRPC[F, C] {

  private lazy val ttlMillis = cacheTtl.toMillis

  private lazy val cacheEnabled = ttlMillis > 0

  private implicit def toOps(contract: C): ContractOps[C] = contractOps(contract)

  // TODO: remove Instant.now() usage
  private def isExpired(cr: ContractRecord[C]): Boolean =
    !cr.contract.nodeParticipates && java.time.Duration.between(cr.lastUpdated, Instant.now()).toMillis >= ttlMillis

  /**
   * Find a contract in local storage
   *
   * @param id Dataset ID
   * @return Optional locally found contract
   */
  override def find(id: Key): F[Option[C]] =
    storage.get(id).attempt.map(_.toOption).flatMap {
      case Some(cr) if isExpired(cr) ⇒
        storage
          .remove(id)
          .map(_ ⇒ None)

      case optCr ⇒
        optCr
          .map(_.contract)
          .filterNot(_.isBlankOffer)
          .pure[F]
    }

  /**
   * Ask to add contract to local storage
   *
   * @param contract Contract to cache
   * @return If the contract is cached or not
   */
  override def cache(contract: C): F[Boolean] =
    if (!contract.canBeCached || !cacheEnabled) {
      false.pure[F]
    } else {
      // We're deciding to cache basing on crypto check, done with canBeCached, and (signed) version number only
      // It allows us to avoid multiplexing network calls with asking to cache stale contracts
      storage.get(contract.id).attempt.map(_.toOption).flatMap {
        case Some(cr) if cr.contract.version < contract.version ⇒ // Contract updated
          storage
            .put(contract.id, contract.record)
            .map(_ ⇒ true)

        case Some(_) ⇒ // Can't update contract with an old version
          false.pure[F]

        case None ⇒ // Contract is unknown, save it
          storage
            .put(contract.id, contract.record)
            .map(_ ⇒ true)
      }
    }
}
