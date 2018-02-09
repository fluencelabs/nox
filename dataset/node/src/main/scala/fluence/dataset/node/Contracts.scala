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

import cats.instances.list._
import cats.syntax.applicative._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.{ Eq, MonadError, Parallel }
import fluence.crypto.signature.{ SignatureChecker, Signer }
import fluence.dataset.contract.{ ContractRead, ContractWrite }
import fluence.dataset.node.contract.ContractRecord
import fluence.dataset.protocol.{ ContractAllocatorRpc, ContractsApi, ContractsCacheRpc }
import fluence.kad.Kademlia
import fluence.kad.protocol.Key
import fluence.storage.KVStore

import scala.concurrent.duration.FiniteDuration
import scala.language.higherKinds
import scala.util.control.NoStackTrace

/**
 * Client-facing API for contracts allocation
 * TODO rename it
 *
 * @param nodeId Current node's Kademlia Key
 * @param storage Cache storage
 * @param createDataset Cluster initialization callback
 * @param checkAllocationPossible Check if allocation is possible
 * @param maxFindRequests Max number of network requests to perform during the find op
 * @param maxAllocateRequests participantsRequired => maxNum of network requests to collect that number of participants
 * @param checker Signature checker
 * @param signer Signer for current node
 * @param cacheTtl Cache TTL
 * @param kademlia Kademlia service
 * @tparam F Effect
 * @tparam Contract Contract
 * @tparam Contact Kademlia's Contact
 */
abstract class Contracts[F[_], Contract : ContractRead : ContractWrite, Contact](
    nodeId: Key,
    storage: KVStore[F, Key, ContractRecord[Contract]],
    createDataset: Contract ⇒ F[Unit],
    checkAllocationPossible: Contract ⇒ F[Unit],
    maxFindRequests: Int,
    maxAllocateRequests: Int ⇒ Int,
    checker: SignatureChecker[F],
    signer: Signer[F],
    cacheTtl: FiniteDuration,
    kademlia: Kademlia[F, Contact]
)(implicit ME: MonadError[F, Throwable], eq: Eq[Contract], P: Parallel[F, F])
  extends ContractsApi[F, Contract] {

  import ContractRead._
  import ContractWrite._

  /**
   * Local cache -- should be used to reply on ContractsCacheRpc requests
   */
  val cache: ContractsCacheRpc[F, Contract] =
    new ContractsCache(nodeId, storage, checker, cacheTtl)

  // Network wrapper -- allows ContractsCacheRpc requests to a remote Contact
  def cacheRpc(contact: Contact): ContractsCacheRpc[F, Contract]

  /**
   * Local allocator -- should be used to reply on ContractAllocatorRpc requests
   */
  val allocator: ContractAllocatorRpc[F, Contract] =
    new ContractAllocator[F, Contract](nodeId, storage, checkAllocationPossible, createDataset, checker, signer)

  // Network wrapper -- allows ContractAllocatorRpc requests to a remote Contact
  def allocatorRpc(contact: Contact): ContractAllocatorRpc[F, Contract]

  /**
   * Search nodes to offer contract, collect participants, allocate dataset on them.
   *
   * @param contract         Contract to allocate
   * @param sealParticipants Client's callback to seal list of participants with a signature
   * @return Sealed contract with a list of participants, or failure
   */
  override def allocate(contract: Contract, sealParticipants: Contract ⇒ F[Contract]): F[Contract] = {
    // Check if contract is already known, return it immediately if it is
    for {
      _ ← ME.ensure(contract.isBlankOffer(checker))(Contracts.IncorrectOfferContract)(identity)
      contractOp ← cache.find(contract.id)
      contract ← contractOp match {
        case Some(c) ⇒ c.pure[F]

        case None ⇒
          kademlia.callIterative[Contract](
            contract.id,
            nc ⇒ allocatorRpc(nc.contact).offer(contract).flatMap { c ⇒
              ME.ensure(c.participantSigned(nc.key, checker))(Contracts.NotFound)(identity) map { _ ⇒ c }
            },
            contract.participantsRequired,
            maxAllocateRequests(contract.participantsRequired),
            isIdempotentFn = false
          ).flatMap {
              case agreements if agreements.lengthCompare(contract.participantsRequired) == 0 ⇒
                contract.addParticipants(checker, agreements.map(_._2))
                  .flatMap { contractToSeal ⇒
                    sealParticipants(contractToSeal)
                  }.flatMap {
                    sealedContract ⇒
                      Parallel.parSequence[List, F, F, Contract](
                        agreements
                          .map(_._1.contact)
                          .map(c ⇒ allocatorRpc(c).allocate(sealedContract)) // In case any allocation failed, failure will be propagated
                          .toList
                      )
                  }.flatMap {
                    case c :: _ ⇒
                      c.pure[F]

                    case Nil ⇒ // Should never happen
                      ME.raiseError[Contract](Contracts.CantFindEnoughNodes(-1))
                  }

              case agreements ⇒
                ME.raiseError[Contract](Contracts.CantFindEnoughNodes(agreements.size))
            }
      }
    } yield contract
  }

  /**
   * Try to find dataset's contract by dataset's kademlia id, or fail.
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
          kademlia.callIterative[Contract](key, nc ⇒ cacheRpc(nc.contact).find(key).flatMap {
            case Some(v) ⇒ v.pure[F]
            case None    ⇒ ME.raiseError(Contracts.NotFound)
          }, 1, maxFindRequests, isIdempotentFn = true).flatMap {
            case sq if sq.nonEmpty ⇒
              // Contract is found; for the case several different versions are returned, find the most recent
              val contract = sq.map(_._2).maxBy(_.version)
              // Cache locally
              cache.cache(contract).map(_ ⇒ contract)

            case _ ⇒
              // Not found -- can't do anything
              ME.raiseError(Contracts.NotFound)
          }
      }

}

object Contracts {

  case object IncorrectOfferContract extends NoStackTrace

  case object NotFound extends NoStackTrace

  case class CantFindEnoughNodes(nodesFound: Int) extends NoStackTrace

}
