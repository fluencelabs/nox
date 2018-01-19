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

import cats.syntax.applicative._
import cats.syntax.applicativeError._
import cats.syntax.eq._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.{ Eq, MonadError }
import fluence.crypto.signature.{ SignatureChecker, Signer }
import fluence.dataset.contract.{ ContractRead, ContractWrite }
import fluence.dataset.node.contract.ContractRecord
import fluence.dataset.protocol.ContractAllocatorRpc
import fluence.kad.protocol.Key
import fluence.storage.KVStore

import scala.language.{ higherKinds, implicitConversions }

/**
 * Performs contracts allocation on local node.
 *
 * @param storage Contracts storage
 * @param createDataset Callback to create a dataset for a successfully allocated contract
 * @param ME Monad error
 * @param eq Contracts equality
 * @tparam F Effect
 * @tparam C Contract
 */
class ContractAllocator[F[_], C : ContractRead : ContractWrite](
    nodeId: Key,
    storage: KVStore[F, Key, ContractRecord[C]],
    createDataset: C ⇒ F[Unit],
    checkAllocationPossible: C ⇒ F[Unit],
    checker: SignatureChecker,
    signer: Signer
)(implicit ME: MonadError[F, Throwable], eq: Eq[C]) extends ContractAllocatorRpc[F, C] {

  import ContractRead._
  import ContractWrite._

  /**
   * Try to allocate a contract.
   *
   * @param contract A sealed contract with all nodes and client signatures
   * @return Allocated contract
   */
  override def allocate(contract: C): F[C] =
    if (!contract.participantSigned(nodeId, checker))
      ME.raiseError(new IllegalArgumentException("Contract should be offered to this node and signed by it prior to allocation"))
    else if (!contract.isActiveContract(checker))
      ME.raiseError(new IllegalArgumentException("Contract should be active -- sealed by client"))
    else
      storage.get(contract.id).attempt.map(_.toOption).flatMap {
        case Some(cr) if !cr.contract.isBlankOffer(checker) ⇒
          cr.contract.pure[F]

        case crOpt ⇒
          for {
            _ ← crOpt.fold(().pure[F])(_ ⇒
              storage.remove(contract.id)
            )
            _ ← checkAllocationPossible(contract)
            _ ← storage.put(contract.id, ContractRecord(contract))
            _ ← createDataset(contract).onError {
              case e ⇒
                storage.remove(contract.id).flatMap(_ ⇒ ME.raiseError(e))
            }
          } yield contract
      }

  /**
   * Offer a contract to node.
   *
   * @param contract A blank contract
   * @return Signed contract, or F is an error
   */
  override def offer(contract: C): F[C] =
    if (!contract.isBlankOffer(checker)) {
      ME.raiseError(new IllegalArgumentException("This is not a valid blank offer"))
    } else {
      def signedContract: C = contract.signOffer(nodeId, signer)

      storage.get(contract.id).attempt.map(_.toOption).flatMap {
        case Some(cr) if cr.contract.isBlankOffer(checker) && cr.contract =!= contract ⇒ // contract preallocated for id, but it's changed now
          for {
            _ ← storage.remove(contract.id)
            _ ← checkAllocationPossible(contract)
            _ ← storage.put(contract.id, ContractRecord(contract))
          } yield signedContract

        case Some(cr) if cr.contract.isBlankOffer(checker) ⇒ // contracts are equal
          signedContract.pure[F]

        case Some(_) ⇒
          ME.raiseError(new IllegalStateException("Different contract is already stored for this ID"))

        case None ⇒
          for {
            _ ← checkAllocationPossible(contract)
            _ ← storage.put(contract.id, ContractRecord(contract))
          } yield signedContract

      }
    }

}
