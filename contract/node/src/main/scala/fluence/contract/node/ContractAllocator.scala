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

import cats.Eq
import cats.data.EitherT
import cats.effect.IO
import cats.syntax.eq._
import cats.syntax.show._
import fluence.contract.ContractError
import fluence.contract.node.ContractsCache._
import fluence.contract.node.cache.ContractRecord
import fluence.contract.ops.{ContractRead, ContractWrite}
import fluence.contract.protocol.ContractAllocatorRpc
import fluence.crypto.signature.SignAlgo.CheckerFn
import fluence.crypto.signature.Signer
import fluence.kad.protocol.Key
import fluence.kvstore.ReadWriteKVStore

import scala.language.{higherKinds, implicitConversions}

/**
 * Performs contracts allocation on local node.
 *
 * @param storage       Contracts storage
 * @param createDataset Callback to create a dataset for a successfully allocated contract
 * @param eq Contracts equality
 * @tparam C Contract
 */
class ContractAllocator[C: ContractRead: ContractWrite](
  nodeId: Key,
  storage: ReadWriteKVStore[Key, ContractRecord[C]],
  createDataset: C ⇒ IO[Boolean],
  checkAllocationPossible: C ⇒ IO[Boolean],
  signer: Signer,
  clock: Clock
)(
  implicit
  eq: Eq[C],
  checkerFn: CheckerFn
) extends ContractAllocatorRpc[C] with slogging.LazyLogging {

  import ContractRead._
  import ContractWrite._

  /**
   * Try to allocate a contract.
   *
   * @param contract A sealed contract with all nodes and client signatures
   * @return Allocated contract
   */
  override def allocate(contract: C): IO[C] = {

    val allocatedContract =
      for {
        _ ← failIfFalse(
          contract.participantSigned(nodeId),
          "Contract should be offered to this node and signed by it prior to allocation"
        )

        _ ← failIfFalse(contract.isActiveContract(), "Contract should be active -- sealed by client")

        contract ← storage
          .get(contract.id)
          .run[IO, ContractError](getError)
          .flatMap {
            case Some(cr) ⇒
              updateIfBlankOffer(contract, cr)
            case None ⇒
              putContract(contract)
          }
      } yield {
        logger.info(
          s"Contract with id=${contract.id.show} was successfully allocated, this node (${nodeId.show}) is contract participant now"
        )
        contract
      }

    allocatedContract.toIO
  }

  private def updateIfBlankOffer(allocatedContract: C, contractFromStorage: ContractRecord[C]) = {
    contractFromStorage.contract
      .isBlankOffer[IO]()
      .flatMap {
        case false ⇒
          EitherT.rightT[IO, ContractError](contractFromStorage.contract)
        case true ⇒
          storage
            .remove(allocatedContract.id)
            .run[IO, ContractError](removeError)
            .flatMap(_ ⇒ putContract(allocatedContract))
      }
  }

  private def putContract(contract: C): EitherT[IO, ContractError, C] = {

    for {

      _ ← failIfFalseIo(checkAllocationPossible(contract), "Allocation is not possible")

      _ ← storage
        .put(contract.id, ContractRecord(contract, clock.instant()))
        .run[IO, ContractError](putError)

      isDsCreated ← EitherT.right[ContractError](createDataset(contract))

      _ ← { // TODO: error should be returned as value
        if (isDsCreated)
          EitherT.right[ContractError](IO.unit)
        else
          storage
            .remove(contract.id)
            .run[IO, ContractError](removeError)
            .subflatMap[ContractError, Unit](_ ⇒ Left(new ContractError("Creation is not possible")))
      }

      _ ← storage
        .get(contract.id)
        .run[IO, ContractError](getError)
        .flatMap[ContractError, ContractRecord[C]] {
          case Some(c) ⇒ EitherT.rightT(c)
          case None ⇒ EitherT.leftT(new ContractError("Creation is not possible"))
        }

    } yield contract

  }

  /**
   * Offer a contract to node.
   *
   * @param contract A blank contract
   * @return Signed contract, or F is an error
   */
  override def offer(contract: C): IO[C] = {
    def signedContract: EitherT[IO, ContractError, C] =
      WriteOps[IO, C](contract)
        .signOffer(nodeId, signer)
        .leftMap(e ⇒ ContractError(e.message))

    val signedAndCheckedContract =
      for {
        _ ← failIfFalse(contract.isBlankOffer[IO](), "This is not a valid blank offer")

        contractOpt ← storage
          .get(contract.id)
          .run[IO, ContractError](getError)

        contract ← contractOpt match {
          case Some(cr) ⇒
            failIfFalse(cr.contract.isBlankOffer(), "Different contract is already stored for this ID").flatMap { _ ⇒
              if (cr.contract =!= contract) {
                // contract preallocated for id, but it's changed now
                logger.debug("preallocated, but changed")
                for {
                  _ ← storage.remove(contract.id).run[IO, ContractError](removeError)
                  ap ← failIfFalseIo(checkAllocationPossible(contract), "Allocation is not possible")
                  _ ← storage
                    .put(contract.id, ContractRecord(contract, clock.instant()))
                    .run[IO, ContractError](putError)
                  sContract ← signedContract
                } yield sContract
              } else {
                // contracts are equal
                signedContract
              }

            }

          case None ⇒
            for {
              ap ← failIfFalseIo(checkAllocationPossible(contract), "Allocation is not possible")
              _ ← storage
                .put(contract.id, ContractRecord(contract, clock.instant()))
                .run[IO, ContractError](putError)
              sContract ← signedContract
            } yield sContract

        }

      } yield {
        logger.info(s"Contract offer with id=${contract.id.show} was accepted with node=${nodeId.show}")
        contract
      }

    signedAndCheckedContract.toIO

  }

  /** If Right(false) - fail EtherT with error msg, return Left(Unit) otherwise */
  private def failIfFalse(
    check: EitherT[IO, ContractError, Boolean],
    msg: String
  ): EitherT[IO, ContractError, Unit] =
    check.flatMap(EitherT.cond[IO](_, (), ContractError(msg)))

  /** If IO(false) - fail IO with error msg, return IO(Unit) value otherwise */
  private def failIfFalseIo(check: IO[Boolean], msg: String): EitherT[IO, ContractError, Unit] = {
    val value = EitherT.right[ContractError](check)
    failIfFalse(value, msg)
  }

  // todo will be removed when all API will be 'EitherT compatible'
  private implicit class EitherT2IO[E <: Throwable, V](origin: EitherT[IO, E, V]) {

    def toIO: IO[V] =
      origin.value.flatMap {
        case Right(value) ⇒ IO.pure(value)
        case Left(error) ⇒ IO.raiseError(error)
      }
  }

}
