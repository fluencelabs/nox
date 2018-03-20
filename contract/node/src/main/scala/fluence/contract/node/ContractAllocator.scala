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

import cats.syntax.applicative._
import cats.syntax.applicativeError._
import cats.syntax.eq._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.syntax.show._
import cats.{Eq, MonadError}
import fluence.contract.ops.{ContractRead, ContractWrite}
import fluence.contract.protocol.ContractAllocatorRpc
import fluence.crypto.signature.{SignatureChecker, Signer}
import fluence.contract.node.cache.ContractRecord
import fluence.kad.protocol.Key
import fluence.storage.KVStore

import scala.language.{higherKinds, implicitConversions}

/**
 * Performs contracts allocation on local node.
 *
 * @param storage       Contracts storage
 * @param createDataset Callback to create a dataset for a successfully allocated contract
 * @param ME            Monad error
 * @param eq            Contracts equality
 * @tparam F Effect
 * @tparam C Contract
 */
class ContractAllocator[F[_], C: ContractRead: ContractWrite](
  nodeId: Key,
  storage: KVStore[F, Key, ContractRecord[C]],
  createDataset: C ⇒ F[Unit],
  checkAllocationPossible: C ⇒ F[Unit],
  signer: Signer,
  clock: Clock
)(
  implicit
  ME: MonadError[F, Throwable],
  eq: Eq[C],
  checker: SignatureChecker
) extends ContractAllocatorRpc[F, C] with slogging.LazyLogging {

  import ContractRead._
  import ContractWrite._

  /**
   * Try to allocate a contract.
   *
   * @param contract A sealed contract with all nodes and client signatures
   * @return Allocated contract
   */
  override def allocate(contract: C): F[C] = {
    for {
      _ ← illegalIfNo(
        contract.participantSigned(nodeId),
        "Contract should be offered to this node and signed by it prior to allocation"
      )
      _ ← illegalIfNo(contract.isActiveContract(), "Contract should be active -- sealed by client")
      contract ← storage.get(contract.id).attempt.map(_.toOption).flatMap {
        case Some(cr) ⇒
          cr.contract.isBlankOffer().flatMap {
            case false ⇒ cr.contract.pure[F]
            case true ⇒ storage.remove(contract.id).flatMap(_ ⇒ putContract(contract))
          }
        case None ⇒
          putContract(contract)
      }
    } yield {
      logger.info(
        s"Contract with id=${contract.id.show} was successfully allocated, this node (${nodeId.show}) is contract participant now"
      )
      contract
    }
  }

  private def putContract(contract: C): F[C] = {
    for {
      _ ← checkAllocationPossible(contract)
      _ ← storage.put(contract.id, ContractRecord(contract, clock.instant()))
      _ ← createDataset(contract).onError {
        case e ⇒
          storage.remove(contract.id).flatMap(_ ⇒ ME.raiseError(e))
      }
    } yield contract
  }

  private def illegalIfNo(condition: F[Boolean], errorMessage: ⇒ String): F[Boolean] = {
    ME.ensure(condition)(new IllegalArgumentException(errorMessage))(identity)
  }

  /**
   * Offer a contract to node.
   *
   * @param contract A blank contract
   * @return Signed contract, or F is an error
   */
  override def offer(contract: C): F[C] = {
    def signedContract: F[C] = contract.signOffer(nodeId, signer)

    for {
      _ ← illegalIfNo(contract.isBlankOffer(), "This is not a valid blank offer")
      contract ← {
        storage.get(contract.id).attempt.map(_.toOption).flatMap {
          case Some(cr) ⇒
            illegalIfNo(cr.contract.isBlankOffer(), "Different contract is already stored for this ID").flatMap { _ ⇒
              if (cr.contract =!= contract) {
                // contract preallocated for id, but it's changed now
                for {
                  _ ← storage.remove(contract.id)
                  _ ← checkAllocationPossible(contract)
                  _ ← storage.put(contract.id, ContractRecord(contract, clock.instant()))
                  sContract ← signedContract
                } yield sContract
              } else {
                // contracts are equal
                signedContract
              }
            }
          case None ⇒
            for {
              _ ← checkAllocationPossible(contract)
              _ ← storage.put(contract.id, ContractRecord(contract, clock.instant()))
              sContract ← signedContract
            } yield sContract
        }
      }
    } yield {
      logger.info(s"Contract offer with id=${contract.id.show} was accepted with node=${nodeId.show}")
      contract
    }
  }

}
