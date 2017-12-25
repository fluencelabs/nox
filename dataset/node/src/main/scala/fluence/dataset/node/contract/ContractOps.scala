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

package fluence.dataset.node.contract

import cats.instances.try_._
import cats.{ Id, MonadError }

import scala.language.higherKinds
import scala.util.Try

/**
 * Common operations for contract allocation and caching.
 *
 * @param contract The contract
 * @param contractSignature Signature checker & signer service
 * @tparam C Contract's type
 */
abstract class ContractOps[C](contract: C, contractSignature: ContractSignature[C])
  extends ContractReadOps[C] with ContractParticipantOps[C] {

  /**
   * Given that this contract is a sealed offer, and contractsWithSigns is a list of signed offers,
   * produce a contract with full list of participants and theirs signatures, or fail if any check fails.
   */
  def collectParticipantSignatures[F[_]](contractsWithSigns: Seq[C])(implicit F: MonadError[F, Throwable]): F[C]

  /**
   * Checks disk space availability.
   *
   * @return Nothing on success, failed F on error
   */
  def checkAllocationPossible[F[_]](implicit F: MonadError[F, Throwable]): F[Unit]

  /**
   * @return true if current node participates in this contract
   */
  def nodeParticipates: Boolean =
    contractSignature.offerSigned[Try](contract).isSuccess

  /**
   * @return Whether this contract is a valid blank offer (with no participants, with client's signature)
   */
  def isBlankOffer: Boolean =
    participants.isEmpty && contractSignature.offerSealed[Try](contract).isSuccess && version == 0

  /**
   * @return Whether this contract offer was signed by this node and client, but participants list is not sealed yet
   */
  def isSignedOffer: Boolean =
    isSignedParticipant && nodeParticipates

  /**
   * @return Whether this contract offer was signed by a single node and client, but participants list is not sealed yet
   */
  def isSignedParticipant: Boolean =
    participants.size == 1 &&
      contractSignature.offerSealed[Try](contract).isSuccess &&
      nodeParticipates &&
      version == 0 &&
      contractSignature.participantsSealed[Try](contract).isFailure

  /**
   * @return Whether this contract is successfully signed by all participants, and participants list is sealed by client
   */
  def isActiveContract: Boolean =
    contractSignature.participantsSealed[Try](contract).isSuccess

  /**
   * Sign a blank offer by current node.
   *
   * @return Signed offer
   */
  def signOffer: C =
    contractSignature.signOffer[Id](contract)

}
