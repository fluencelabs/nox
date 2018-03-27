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

import cats.data.EitherT
import cats.effect.LiftIO
import cats.instances.list._
import cats.syntax.functor._
import cats.syntax.show._
import cats.{Eq, Monad, Parallel, Show}
import fluence.contract.ops.{ContractRead, ContractWrite}
import fluence.contract.protocol.{ContractAllocatorRpc, ContractsCacheRpc}
import fluence.crypto.algorithm.CryptoErr
import fluence.crypto.signature.SignatureChecker
import fluence.kad.Kademlia
import fluence.kad.protocol.Key

import scala.language.higherKinds

trait Contracts[F[_], Contract] {

  /**
   * Search nodes to offer contract, collect participants, allocate dataset on them.
   *
   * @param contract         Contract to allocate
   * @param sealParticipants Client's callback to seal list of participants with a signature
   * @return Sealed contract with a list of participants, or failure
   */
  def allocate(
    contract: Contract,
    sealParticipants: Contract ⇒ EitherT[F, String, Contract]
  ): EitherT[F, Contracts.AllocateError, Contract]

  /**
   * Try to find dataset's contract by dataset's kademlia id, or fail.
   *
   * @param key Dataset ID
   */
  def find(key: Key): EitherT[F, Contracts.AllocateError, Contract]
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
  def apply[F[_]: LiftIO: Monad, G[_], Contract: ContractRead: ContractWrite, Contact](
    maxFindRequests: Int,
    maxAllocateRequests: Int ⇒ Int,
    kademlia: Kademlia[F, Contact],
    cacheRpc: Contact ⇒ ContractsCacheRpc[Contract],
    allocatorRpc: Contact ⇒ ContractAllocatorRpc[Contract]
  )(
    implicit
    eq: Eq[Contract],
    P: Parallel[F, G],
    checker: SignatureChecker,
    show: Show[Contact]
  ): Contracts[F, Contract] = new Contracts[F, Contract] with slogging.LazyLogging {

    import ContractRead._
    import ContractWrite._

    /**
     * Search nodes to offer contract, collect participants, allocate dataset on them.
     *
     * @param contract         Contract to allocate
     * @param sealParticipants Client's callback to seal list of participants with a signature
     * @return Sealed contract with a list of participants, or failure
     */
    override def allocate(
      contract: Contract,
      sealParticipants: Contract ⇒ EitherT[F, String, Contract]
    ): EitherT[F, AllocateError, Contract] =
      // Check if contract is already known, return it immediately if it is
      for {
        _ ← contract
          .isBlankOffer[F]()
          .leftMap(CryptoError)
          .subflatMap(Either.cond[AllocateError, Unit](_, (), Contracts.IncorrectOfferContract))

        agreements ← EitherT.right[AllocateError](
          kademlia
            .callIterative[AllocateError, Contract](
              contract.id,
              nc ⇒
                EitherT(allocatorRpc(nc.contact).offer(contract).attempt.to[F])
                  .leftMap(_ ⇒ Contracts.NotFound)
                  .flatMap { contract ⇒
                    contract
                      .participantSigned[F](nc.key)
                      .leftMap(CryptoError)
                      .subflatMap(Either.cond(_, contract, Contracts.NotFound))
                },
              contract.participantsRequired,
              maxAllocateRequests(contract.participantsRequired),
              isIdempotentFn = false
            )
        )

        _ ← EitherT.cond[F](
          agreements.lengthCompare(contract.participantsRequired) == 0,
          (),
          Contracts.CantFindEnoughNodes(agreements.size): AllocateError
        )

        _ = logger.debug(s"Agreements for contract $contract found. Contacts: ${agreements.map(_._1.contact.show)}")

        contractToSeal ← WriteOps[F, Contract](contract)
          .addParticipants(agreements.map(_._2))
          .leftMap[AllocateError](CryptoError)

        sealedContract ← sealParticipants(contractToSeal).leftMap[AllocateError](ClientError)

        allocated ← EitherT.right[AllocateError](
          Parallel.parSequence[List, F, G, Contract]( // TODO: failure handling could be incorrect
            agreements
              .map(_._1.contact)
              .map(c ⇒ allocatorRpc(c).allocate(sealedContract).to[F]) // In case any allocation failed, failure will be propagated
              .toList
          )
        )

        finalContract ← EitherT
          .fromOption[F](allocated.headOption, Contracts.CantFindEnoughNodes(-1): AllocateError) // TODO: this should never fail; however, error message should be descriptive

      } yield finalContract

    /**
     * Try to find dataset's contract by dataset's kademlia id, or fail.
     *
     * @param key Dataset ID
     */
    override def find(key: Key): EitherT[F, AllocateError, Contract] =
      // Try to lookup in the neighborhood
      // TODO: if contract is found "too far" from the neighborhood, ask key's neighbors to cache contract
      EitherT(
        kademlia
          .callIterative[AllocateError, Contract](
            key,
            nc ⇒
              EitherT(cacheRpc(nc.contact).find(key).attempt.to[F].map[Either[AllocateError, Contract]] {
                case Right(Some(v)) ⇒ Right(v)
                case o ⇒ Left(Contracts.NotFound)
              }),
            1,
            maxFindRequests,
            isIdempotentFn = true
          )
          .map {
            case sq if sq.nonEmpty ⇒
              // Contract is found; for the case several different versions are returned, find the most recent
              Right(sq.map(_._2).maxBy(_.version))

            case _ ⇒
              // Not found -- can't do anything
              Left(Contracts.NotFound)
          }
      )

  }

  sealed trait AllocateError

  // TODO: this wrapper is ugly, but if we want to avoid use of CoFail, what else could we do?
  case class CryptoError(err: CryptoErr) extends AllocateError

  case object IncorrectOfferContract extends AllocateError

  case object NotFound extends AllocateError

  case class CantFindEnoughNodes(nodesFound: Int) extends AllocateError

  case class ClientError(message: String) extends AllocateError

}
