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

package fluence.contract.client

import cats.instances.list._
import cats.syntax.applicative._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.syntax.show._
import cats.{Eq, MonadError, Parallel, Show}
import fluence.contract.ops.{ContractRead, ContractWrite}
import fluence.contract.protocol.{ContractAllocatorRpc, ContractsCacheRpc}
import fluence.crypto.signature.SignatureChecker
import fluence.kad.Kademlia
import fluence.kad.protocol.Key

import scala.language.higherKinds
import scala.util.control.NoStackTrace

trait Contracts[F[_], Contract] {

  /**
   * Search nodes to offer contract, collect participants, allocate dataset on them.
   *
   * @param contract         Contract to allocate
   * @param sealParticipants Client's callback to seal list of participants with a signature
   * @return Sealed contract with a list of participants, or failure
   */
  def allocate(contract: Contract, sealParticipants: Contract ⇒ F[Contract]): F[Contract]

  /**
   * Try to find dataset's contract by dataset's kademlia id, or fail.
   *
   * @param key Dataset ID
   */
  def find(key: Key): F[Contract]
}

object Contracts {

  /**
   * Client-facing API for contracts allocation
   *
   * @param maxFindRequests Max number of network requests to perform during the find op
   * @param maxAllocateRequests participantsRequired => maxNum of network requests to collect that number of participants
   * @param checker Signature checker
   * @param kademlia Kademlia service
   * @tparam F Effect
   * @tparam Contract Contract
   * @tparam Contact Kademlia's Contact
   */
  def apply[F[_], G[_], Contract: ContractRead: ContractWrite, Contact](
    maxFindRequests: Int,
    maxAllocateRequests: Int ⇒ Int,
    kademlia: Kademlia[F, Contact],
    cacheRpc: Contact ⇒ ContractsCacheRpc[F, Contract],
    allocatorRpc: Contact ⇒ ContractAllocatorRpc[F, Contract]
  )(
    implicit ME: MonadError[F, Throwable],
    eq: Eq[Contract],
    P: Parallel[F, G],
    checker: SignatureChecker,
    show: Show[Contact]
  ): Contracts[F, Contract] = new Contracts[F, Contract] with slogging.LazyLogging {

    import ContractRead._
    import ContractWrite._

    // TODO: return error in EitherT, never use .raise nor throw
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
        _ ← ME.ensure(contract.isBlankOffer())(Contracts.IncorrectOfferContract)(identity)
        contract ← kademlia
          .callIterative[Contract](
            contract.id,
            nc ⇒
              allocatorRpc(nc.contact).offer(contract).flatMap { c ⇒
                ME.ensure(c.participantSigned(nc.key))(Contracts.NotFound)(identity) map { _ ⇒
                  c
                }
            },
            contract.participantsRequired,
            maxAllocateRequests(contract.participantsRequired),
            isIdempotentFn = false
          )
          .flatMap {
            case agreements if agreements.lengthCompare(contract.participantsRequired) == 0 ⇒
              logger.debug(s"Agreements for contract $contract found. Contacts: ${agreements.map(_._1.contact.show)}")
              contract
                .addParticipants(agreements.map(_._2))
                .flatMap { contractToSeal ⇒
                  sealParticipants(contractToSeal)
                }
                .flatMap { sealedContract ⇒
                  Parallel.parSequence[List, F, G, Contract](
                    agreements
                      .map(_._1.contact)
                      .map(c ⇒ allocatorRpc(c).allocate(sealedContract)) // In case any allocation failed, failure will be propagated
                      .toList
                  )
                }
                .flatMap {
                  case c :: _ ⇒
                    c.pure[F]

                  case Nil ⇒ // Should never happen
                    ME.raiseError[Contract](Contracts.CantFindEnoughNodes(-1))
                }

            case agreements ⇒
              ME.raiseError[Contract](Contracts.CantFindEnoughNodes(agreements.size))
          }
      } yield contract
    }

    /**
     * Try to find dataset's contract by dataset's kademlia id, or fail.
     *
     * @param key Dataset ID
     */
    override def find(key: Key): F[Contract] =
      // Try to lookup in the neighborhood
      // TODO: if contract is found "too far" from the neighborhood, ask key's neighbors to cache contract
      kademlia
        .callIterative[Contract](
          key,
          nc ⇒
            cacheRpc(nc.contact).find(key).flatMap {
              case Some(v) ⇒ v.pure[F]
              case None ⇒ ME.raiseError(Contracts.NotFound)
          },
          1,
          maxFindRequests,
          isIdempotentFn = true
        )
        .flatMap {
          case sq if sq.nonEmpty ⇒
            // Contract is found; for the case several different versions are returned, find the most recent
            sq.map(_._2).maxBy(_.version).pure[F]

          case _ ⇒
            // Not found -- can't do anything
            ME.raiseError(Contracts.NotFound)
        }

  }

  case object IncorrectOfferContract extends NoStackTrace

  case object NotFound extends NoStackTrace

  case class CantFindEnoughNodes(nodesFound: Int) extends NoStackTrace

}
