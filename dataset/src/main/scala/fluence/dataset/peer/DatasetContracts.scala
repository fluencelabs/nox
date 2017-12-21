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

package fluence.dataset.peer

import cats.{ Eq, MonadError, Parallel }
import cats.syntax.flatMap._
import cats.syntax.applicative._
import cats.syntax.functor._
import cats.syntax.applicativeError._
import cats.instances.list._
import fluence.dataset.protocol._
import fluence.kad.{ Kademlia, Key }
import fluence.node.storage.KVStore

import scala.concurrent.duration.FiniteDuration
import scala.language.higherKinds
import scala.util.control.NoStackTrace

abstract class DatasetContracts[F[_], Contract, Contact](
    storage: KVStore[F, Key, ContractRecord[Contract]],
    contractOps: Contract ⇒ ContractOps[Contract],
    createDataset: Contract ⇒ F[Unit],
    maxFindRequests: Int,
    maxAllocateRequests: Int ⇒ Int,
    cacheTtl: FiniteDuration,
    kademlia: Kademlia[F, Contact]
)(implicit ME: MonadError[F, Throwable], eq: Eq[Contract], P: Parallel[F, F])
  extends ContractsAllocatorApi[F, Contract] {

  val cache: ContractsCacheRpc[F, Contract] =
    new ContractsCache(storage, contractOps, cacheTtl)

  def cacheRpc(contact: Contact): ContractsCacheRpc[F, Contract]

  val allocator: ContractAllocatorRpc[F, Contract] =
    new ContractAllocator[F, Contract](storage, contractOps, createDataset)

  def allocatorRpc(contact: Contact): ContractAllocator[F, Contract]

  override def allocate(contract: Contract, sealParticipants: Contract ⇒ F[Contract]): F[Contract] = {
    // Check if contract is already known, return it immediately if it is
    val co = contractOps(contract)
    import co.{ id, participantsRequired }
    cache.find(id).flatMap {
      case Some(c) ⇒ c.pure[F]

      case None ⇒
        kademlia.callIterative[Contract](
          id,
          nc ⇒ allocatorRpc(nc.contact).offer(contract), // TODO: check contract signatures in response
          participantsRequired,
          maxAllocateRequests(participantsRequired),
          isIdempotentFn = false
        ).flatMap {
            case agreements if agreements.lengthCompare(participantsRequired) == 0 ⇒
              val contractToSeal = agreements.foldLeft(contract){
                case (acc, (_, add)) ⇒ add // TODO: merge participants
              }
              sealParticipants(contractToSeal).flatMap {
                sealedContract ⇒
                  Parallel.parSequence[List, F, F, Either[Throwable, Contract]](
                    agreements
                      .map(_._1.contact)
                      .map(c ⇒ allocatorRpc(c).allocate(sealedContract).attempt)
                      .toList
                  )
              }.flatMap {
                case signatures if signatures.nonEmpty && signatures.forall(_.isRight) ⇒
                  // TODO: this is ugly!
                  signatures.head.toOption.get.pure[F]

                case signatures ⇒
                  // TODO: what to do if some nodes signed an offer, but didn't perform allocation?
                  ME.raiseError(DatasetContracts.CantFindEnoughNodes(signatures.count(_.isRight)))
              }

            case agreements ⇒
              ME.raiseError(DatasetContracts.CantFindEnoughNodes(agreements.size))
          }
    }
  }

  /**
   * Try to find dataset's contract by dataset's kademlia id, or fail
   *
   * @param key Dataset ID
   */
  override def find(key: Key): F[Contract] =
    // Lookup local cache
    cache.find(key)
      .flatMap {
        case Some(v) ⇒
          // Found -- return
          v.pure[F]
        case None ⇒
          // Try to lookup in the neighborhood
          // TODO: if contract is found "too far" from the neighborhood, ask key's neighbors to cache contract
          kademlia.callIterative[Contract](key, nc ⇒ cacheRpc(nc.contact).find(key).flatMap{
            case Some(v) ⇒ v.pure[F]
            case None    ⇒ ME.raiseError(DatasetContracts.NotFound)
          }, 1, maxFindRequests, isIdempotentFn = true).flatMap{
            case sq if sq.nonEmpty ⇒
              // Contract is found; for the case several different versions are returned, find the most recent
              val contract = sq.map(_._2).maxBy(contractOps(_).version)
              // Cache locally
              cache.cache(contract).map(_ ⇒ contract)

            case _ ⇒
              // Not found -- can't do anything
              ME.raiseError(DatasetContracts.NotFound)
          }
      }

}

object DatasetContracts {
  case object NotFound extends NoStackTrace
  case class CantFindEnoughNodes(nodesFound: Int) extends NoStackTrace
}
