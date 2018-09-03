/*
 * Copyright (C) 2018  Fluence Labs Limited
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

package fluence.statemachine.tx

import cats.Monad
import cats.data.EitherT
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.statemachine.StoreValue
import fluence.statemachine.error.StateMachineError
import fluence.statemachine.state.MutableStateTree
import fluence.statemachine.tree.StorageKeys.{payloadKey, resultKey, statusKey}
import slogging.LazyLogging

import scala.language.higherKinds

/**
 * Performs a series of actions with successfully validated (successfully parsed and unique) transaction received
 * in `DeliverTx` request.
 *
 * Any transaction received by [[TxProcessor]] affects the application state since the processing start,
 * even if transaction would be rejected by VM or it's invocation would be failed.
 * It also means that Consensus tree root's hash (interpreted as `app_hash` on `Commit`) is almost certainly changed
 * (except rare cases of collisions).
 *
 * Note that `DeliverTx` processing is synchronous, `DeliverTxResponse` sending is performed only after
 * transaction appending, invocation and even invocation of dependent, previously queued transactions.
 *
 * @param mutableConsensusState Consensus state, affected by every newly appended transaction
 * @param vmInvoker object used to perform immediate calls to VM
 */
class TxProcessor[F[_]](
  private val mutableConsensusState: MutableStateTree[F],
  private val vmInvoker: VmOperationInvoker[F]
)(implicit F: Monad[F])
    extends LazyLogging {

  /**
   * Enqueues given `tx` and tries to apply it immediately.
   * Otherwise this transaction remains queued and not applied until required transaction becomes successfully applied.
   *
   * As soon as any transaction becomes successfully applied it tries to apply the dependent transaction if it is
   * queued.
   *
   * @param tx transaction ready to be processed
   */
  def processNewTx(tx: Transaction): F[Unit] =
    for {
      _ <- enqueueTx(tx)
      _ = logger.debug("Appended tx: {}", tx)
      _ <- tx.header.requiredTxHeader match {
        case None => applyTxWithDependencies(tx.header)
        case Some(required) =>
          for {
            requiredSuccess <- getStatus(required).map(x => x.contains(TransactionStatus.Success))
            _ <- if (requiredSuccess) applyTxWithDependencies(tx.header) else F.unit
          } yield ()
      }
    } yield ()

  /**
   * Triggers the application of a transaction stored by given header (if actually stored).
   * In case of success triggers the application of dependent transactions that might be queued
   * (if received out of order previously).
   *
   * @param txHeader header of the transaction to be applied
   */
  private def applyTxWithDependencies(txHeader: TransactionHeader): F[Unit] =
    for {
      // Collecting tx and previously queued dependencies (if any)
      txs <- txAndQueuedDependencies(txHeader)
      _ <- txs
      // Preparing txs for application. It includes invocation and dequeuing
        .map(tx => {
          for {
            invoked <- invokeTx(tx).isRight
            _ <- dequeueTx(tx)
          } yield invoked
        })
        // Applying prepared txs while this application is successful
        .foldLeft(F.pure(true))((acc, invoked) => {
          for {
            successBefore <- acc
            successAfter <- if (successBefore) invoked else F.pure(false)
          } yield successAfter
        })
        .map(_ => ())
    } yield ()

  /**
   * Builds list of transactions including given transaction and its possibly queued dependent transactions.
   *
   * @param txHeader header of the transaction to be applied
   */
  private def txAndQueuedDependencies(txHeader: TransactionHeader): F[List[Transaction]] =
    for {
      payloadOp <- getStoredPayload(txHeader)
      result <- payloadOp match {
        case None => F.pure(Nil)
        case Some(payload) =>
          txAndQueuedDependencies(txHeader.dependentTxHeader).map(Transaction(txHeader, payload) :: _)
      }
    } yield result

  /**
   * Effectively invokes the given transaction and writes it's results to Consensus state.
   *
   * @param tx transaction ready to be applied (by the invocation in VM)
   */
  private def invokeTx(tx: Transaction): EitherT[F, StateMachineError, Option[String]] =
    EitherT(for {
      invoked <- vmInvoker.invoke(tx.payload).value
      _ <- invoked match {
        case Left(error) => putResult(tx, TransactionStatus.Error, Error(error.code, error.message))
        case Right(None) => putResult(tx, TransactionStatus.Success, Empty)
        case Right(Some(value)) => putResult(tx, TransactionStatus.Success, Computed(value))
      }
    } yield invoked)

  private def enqueueTx(tx: Transaction): F[Unit] =
    for {
      _ <- mutableConsensusState.putValue(payloadKey(tx.header), tx.payload)
      _ <- mutableConsensusState.putValue(statusKey(tx.header), TransactionStatus.Queued)
    } yield ()

  private def dequeueTx(tx: Transaction): F[Unit] =
    for {
      _ <- mutableConsensusState.removeValue(payloadKey(tx.header))
    } yield ()

  private def getStatus(txHeader: TransactionHeader): F[Option[String]] =
    for {
      root <- mutableConsensusState.getRoot
      value = root.getValue(statusKey(txHeader))
    } yield value

  private def getStoredPayload(txHeader: TransactionHeader): F[Option[String]] =
    for {
      root <- mutableConsensusState.getRoot
      value = root.getValue(payloadKey(txHeader))
    } yield value

  private def putResult(tx: Transaction, status: StoreValue, result: TxInvocationResult): F[Unit] = {
    for {
      _ <- mutableConsensusState.putValue(resultKey(tx.header), result.toStoreValue)
      _ <- mutableConsensusState.putValue(statusKey(tx.header), status)
    } yield ()
  }
}
