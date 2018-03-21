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

import cats.data.EitherT
import cats.effect.IO
import cats.syntax.applicative._
import cats.syntax.eq._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.syntax.show._
import cats.{~>, Eq, Monad}
import fluence.contract.ops.{ContractRead, ContractWrite}
import fluence.contract.protocol.ContractAllocatorRpc
import fluence.crypto.signature.{SignatureChecker, Signer}
import fluence.contract.node.cache.ContractRecord
import fluence.crypto.algorithm.CryptoErr
import fluence.kad.protocol.Key
import fluence.storage.KVStore

import scala.language.{higherKinds, implicitConversions}

/**
 * Performs contracts allocation on local node.
 * TODO: we have a number of toIO convertions due to wrong [[KVStore.get]] signature; it should be fixed
 *
 * @param storage Contracts storage
 * @param createDataset Callback to create a dataset for a successfully allocated contract
 * @param eq Contracts equality
 * @tparam F Effect
 * @tparam C Contract
 */
class ContractAllocator[F[_]: Monad, C: ContractRead: ContractWrite](
  nodeId: Key,
  storage: KVStore[F, Key, ContractRecord[C]],
  createDataset: C ⇒ F[Boolean],
  checkAllocationPossible: C ⇒ F[Boolean],
  signer: Signer,
  toIO: F ~> IO
)(
  implicit
  eq: Eq[C],
  checker: SignatureChecker
) extends ContractAllocatorRpc[C] with slogging.LazyLogging {

  import ContractRead._
  import ContractWrite._

  /**
   * Try to allocate a contract.
   *
   * @param contract A sealed contract with all nodes and client signatures
   * @return Allocated contract
   */
  override def allocate(contract: C): IO[C] =
    for {
      _ ← eitherFail(
        contract.participantSigned(nodeId),
        "Contract should be offered to this node and signed by it prior to allocation"
      )
      _ ← eitherFail(contract.isActiveContract(), "Contract should be active -- sealed by client")

      contract ← toIO(storage.get(contract.id)).attempt.map(_.toOption).flatMap {
        case Some(cr) ⇒
          cr.contract.isBlankOffer[IO]().value.map(_.contains(true)).flatMap {
            case false ⇒ cr.contract.pure[IO]
            case true ⇒ toIO(storage.remove(contract.id).flatMap(_ ⇒ putContract(contract)))
          }
        case None ⇒
          toIO(putContract(contract))
      }
    } yield {
      logger.info(
        s"Contract with id=${contract.id.show} was successfully allocated, this node (${nodeId.show}) is contract participant now"
      )
      contract
    }

  private def putContract(contract: C): F[C] =
    for {
      _ ← checkAllocationPossible(contract)
      _ ← storage.put(contract.id, ContractRecord(contract))
      created ← createDataset(contract)
      // TODO: error should be returned as value
      _ ← if (created) ().pure[F] else storage.remove(contract.id)
    } yield contract

  private def eitherFail(check: EitherT[IO, CryptoErr, Boolean], msg: String): IO[Unit] =
    check
      .leftMap(_.errorMessage)
      .flatMap(
        EitherT.cond[IO](_, (), msg)
      )
      .leftMap(new RuntimeException(_))
      .value
      .flatMap(IO.fromEither)

  /**
   * Offer a contract to node.
   *
   * @param contract A blank contract
   * @return Signed contract, or F is an error
   */
  override def offer(contract: C): IO[C] = {
    def signedContract: IO[C] =
      WriteOps[IO, C](contract)
        .signOffer(nodeId, signer)
        .leftMap(_.errorMessage)
        .leftMap(new RuntimeException(_))
        .value
        .flatMap(IO.fromEither)

    for {
      _ ← eitherFail(contract.isBlankOffer[IO](), "This is not a valid blank offer")

      contractOpt ← toIO(storage.get(contract.id)).attempt.map(_.toOption)

      contract ← contractOpt match {
        case Some(cr) ⇒
          eitherFail(cr.contract.isBlankOffer(), "Different contract is already stored for this ID").flatMap { _ ⇒
            if (cr.contract =!= contract) {
              // contract preallocated for id, but it's changed now
              for {
                _ ← toIO(storage.remove(contract.id))
                _ ← toIO(checkAllocationPossible(contract))
                _ ← toIO(storage.put(contract.id, ContractRecord(contract)))
                sContract ← signedContract
              } yield sContract
            } else {
              // contracts are equal
              signedContract
            }
          }

        case None ⇒
          for {
            _ ← toIO(checkAllocationPossible(contract))
            _ ← toIO(storage.put(contract.id, ContractRecord(contract)))
            sContract ← signedContract
          } yield sContract
      }

    } yield {
      logger.info(s"Contract offer with id=${contract.id.show} was accepted with node=${nodeId.show}")
      contract
    }
  }

}
