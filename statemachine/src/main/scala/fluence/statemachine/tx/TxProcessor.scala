/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package fluence.statemachine.tx

import cats.Monad
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.statemachine.state.MutableStateTree
import fluence.statemachine.tree.StoragePaths._
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
      _ <- incrementTxCounter()
      _ <- enqueueTx(tx)
      _ = logger.debug("Appended tx: {}", tx)
      _ <- tx.header.requiredTxHeader match {
        case None => applyTxWithDependencies(tx.header)
        case Some(required) =>
          for {
            requiredStatus <- getStatus(required)
            _ <- if (requiredStatus.exists(_.allowDependentTxInvocation)) applyTxWithDependencies(tx.header) else F.unit
          } yield ()
      }
      _ <- markExpiredSessions()
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
            invoked <- invokeTx(tx).map(_.allowDependentTxInvocation)
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
   * @return [[TransactionStatus]] corresponding to the invocation result
   */
  private def invokeTx(tx: Transaction): F[TransactionStatus] = tx.payload match {
    case "@closeSession()" => putResult(tx, TransactionStatus.SessionClosed, Empty, ExplicitlyClosed)
    case _ =>
      for {
        //callDescription <- FunctionCallDescription.parse[F](tx.payload)
        //invoked <- vmInvoker.invoke(callDescription).value
        invoked <- FunctionCallDescription.parse[F](tx.payload).flatMap(x => vmInvoker.invoke(x)).value
        txStatus <- invoked match {
          case Left(error) => putResult(tx, TransactionStatus.Error, Error(error.code, error.message), Failed)
          case Right(None) => putResult(tx, TransactionStatus.Success, Empty, Active)
          case Right(Some(value)) => putResult(tx, TransactionStatus.Success, Computed(value), Active)
        }
      } yield txStatus
  }

  /**
   * Traverses all currently stored sessions and mark those of them which need to be expired,
   * i. e. still with [[Active]] status, but without any transaction processing during `inactivity` period.
   *
   */
  private def markExpiredSessions(): F[Unit] =
    for {
      root <- mutableConsensusState.getRoot
      txCounter <- getTxCounter
      statusKeys = root.selectByTemplate(SessionSummarySelector)
      expirationList = statusKeys.flatMap(
        statusKey =>
          root
            .getValue(statusKey)
            .flatMap(SessionSummary.fromStoreValue)
            .filter(summary => summary.status == Active && summary.lastTxCounter <= txCounter - 8)
            .map(summary => mutableConsensusState.putValue(statusKey, summary.copy(status = Expired).toStoreValue))
      )
      _ <- expirationList.foldLeft(F.unit)((acc, expiration) => {
        for {
          _ <- acc
          _ <- expiration
        } yield ()
      })
    } yield ()

  private def getTxCounter: F[Long] =
    for {
      root <- mutableConsensusState.getRoot
      value = root.getValue(TxCounterPath).map(_.toLong).getOrElse(0L)
    } yield value

  private def incrementTxCounter(): F[Long] =
    for {
      oldValue <- getTxCounter
      newValue = oldValue + 1
      _ <- mutableConsensusState.putValue(TxCounterPath, newValue.toString)
    } yield newValue

  private def enqueueTx(tx: Transaction): F[Unit] =
    for {
      _ <- mutableConsensusState.putValue(txPayloadPath(tx.header), tx.payload)
      _ <- mutableConsensusState.putValue(txStatusPath(tx.header), TransactionStatus.Queued.storeValue)
    } yield ()

  private def dequeueTx(tx: Transaction): F[Unit] =
    for {
      _ <- mutableConsensusState.removeValue(txPayloadPath(tx.header))
    } yield ()

  private def getStatus(txHeader: TransactionHeader): F[Option[TransactionStatus]] =
    for {
      root <- mutableConsensusState.getRoot
      value = root.getValue(txStatusPath(txHeader)).flatMap(TransactionStatus.fromStoreValue)
    } yield value

  private def getStoredPayload(txHeader: TransactionHeader): F[Option[String]] =
    for {
      root <- mutableConsensusState.getRoot
      value = root.getValue(txPayloadPath(txHeader))
    } yield value

  private def putResult(
    tx: Transaction,
    txStatus: TransactionStatus,
    result: TxInvocationResult,
    sessionStatus: SessonStatus
  ): F[TransactionStatus] = {
    for {
      _ <- mutableConsensusState.putValue(txResultPath(tx.header), result.toStoreValue)
      _ <- mutableConsensusState.putValue(txStatusPath(tx.header), txStatus.storeValue)
      txCounter <- getTxCounter
      sessionSummary = SessionSummary(sessionStatus, tx.header.order + 1, txCounter).toStoreValue
      _ <- mutableConsensusState.putValue(sessionSummaryPath(tx.header), sessionSummary)
    } yield txStatus
  }
}
