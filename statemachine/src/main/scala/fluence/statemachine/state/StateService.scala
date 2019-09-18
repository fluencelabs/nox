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

package fluence.statemachine.state

import cats.effect.Effect
import cats.effect.concurrent.Ref
import cats.instances.option._
import cats.syntax.applicative._
import cats.syntax.apply._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.syntax.option._
import cats.syntax.compose._
import cats.{Applicative, Eval, Monad, Traverse}
import fluence.bp.tx.{Tx, TxCode, TxResponse}
import fluence.crypto.Crypto
import fluence.crypto.Crypto.Hasher
import fluence.crypto.hash.JdkCryptoHasher
import fluence.log.Log
import fluence.statemachine.api.query.{QueryCode, QueryResponse}
import fluence.statemachine.api.data.{BlockReceipt, StateHash}
import fluence.statemachine.receiptbus.ReceiptBusBackend
import fluence.statemachine.vm.VmOperationInvoker
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * Wraps all the state and logic required to perform ABCI logic.
 *
 * @param state See [[MachineState]]
 * @param vm Virtual machine invoker
 * @param receiptBusBackend Communication channel with master node
 */
class StateService[F[_]: Monad](
  state: Ref[F, MachineState],
  vm: VmOperationInvoker[F],
  receiptBusBackend: ReceiptBusBackend[F]
)(implicit hasher: Hasher[ByteVector, ByteVector], log: Log[F]) {

  // Writes a trace log about block uploading
  private def traceBU(msg: String)(implicit log: Log[F]) =
    log.trace(Console.YELLOW + s"BUD: $msg" + Console.RESET)

  /**
   * Get actual stateHash from the internal state
   */
  def stateHash: F[StateHash] = state.get.map(_.stateHash)

  /**
   * Take all the transactions we're able to process, and pass them to VM one by one.
   *
   * @return App Hash (with receipt hash applied!)
   */
  def commit(implicit log: Log[F]): F[StateHash] =
    for {
      // Get current state
      currentState <- state.get
      // Form a block: take ordered txs from AbciState
      sTxs @ (_, transactions) ← MachineState.formBlock[F].run(currentState)

      // Process txs one by one
      st ← Monad[F].tailRecM[(MachineState, List[Tx]), MachineState](sTxs) {
        case (st, tx :: txs) ⇒
          // Invoke
          vm.invoke(tx.data.value)
            // Save the tx response to AbciState
            .semiflatMap(result ⇒ MachineState.putResponse[F](tx.head, result.output).map(_ ⇒ txs).run(st).map(Left(_)))
            .leftSemiflatMap(err ⇒ Log[F].error(s"VM invoke failed: $err for tx: $tx").as(err))
            .getOrElse(Right(st)) // TODO do not ignore vm error

        case (st, Nil) ⇒
          Applicative[F].pure(Right(st))
      }

      blockHeight = st.height + 1

      // Get the VM hash
      vmHash ← vm
        .vmStateHash()
        .leftSemiflatMap(err ⇒ Log[F].error(s"VM is unable to compute state hash: $err").as(err))
        .getOrElse(ByteVector.empty) // TODO do not ignore vm error

      _ <- traceBU(s"got vmHash; height ${st.height + 1}" + Console.RESET)

      // Do not wait for receipt on empty blocks
      receipt <- if (receiptBusBackend.isEnabled && transactions.nonEmpty) {
        traceBU(s"retrieving receipt on height $blockHeight" + Console.RESET) *>
          receiptBusBackend.getReceipt(blockHeight - 1).map(_.some)
      } else {
        traceBU(s"WON'T retrieve receipt on height $blockHeight" + Console.RESET) *>
          none[BlockReceipt].pure[F]
      }

      _ <- traceBU(
        s"got receipt ${receipt.map(r => s"${r.height}")}; " +
          s"transactions count: ${transactions.length}"
      )

      _ <- Traverse[Option].traverse(receipt.filter(_.height != blockHeight - 1))(
        b =>
          log.error(
            s"Got wrong receipt height. current height: $blockHeight, receipt: ${b.height} (expected ${blockHeight - 1})"
          )
      )

      // Do not use receipt in app hash if there's no txs in a block, so empty blocks have the same appHash as
      // previous non-empty ones. This is because Tendermint stops producing empty blocks only after
      // at least 2 blocks have the same appHash. Otherwise, empty blocks would be produced indefinitely.
      appHash <- receipt.fold {
        if (!receiptBusBackend.isEnabled || blockHeight == 1)
          // To save initial state of VM in a block chain and also to make it produce 2 blocks on the start
          vmHash.pure[F]
        else
          currentState.appHash.pure[F]
      } {
        case BlockReceipt(_, bytes) =>
          traceBU(s"appHash = hash(${vmHash.toHex} ++ ${bytes.toHex})") *>
            hasher[F](vmHash ++ bytes)
              .leftMap(err => log.error(s"Error on hashing vmHash + receipt: $err"))
              .getOrElse(vmHash) // TODO: don't ignore errors
      }

      // Push hash to AbciState, increment block number
      newState ← MachineState.setAppHash[F](appHash).runS(st)

      // Store updated state in the Ref (the changes were transient for readers before this step)
      _ ← state.set(newState)

      _ <- traceBU("state.set done")

      stateHash = StateHash(blockHeight, appHash)

      // Store vmHash
      _ <- receiptBusBackend.enqueueVmHash(blockHeight, vmHash)
      _ <- log.info(s"$blockHeight commit end")
    } yield stateHash

  /**
   * Queries the storage for sessionId/nonce result, or for sessionId status.
   *
   * @param path sessionId/nonce or sessionId
   */
  def query(path: String): F[QueryResponse] =
    Tx.readHead(path) match {
      // There's no /nonce part, but path could be a sessionId as a whole
      case None ⇒
        state.get.map(
          state ⇒
            QueryResponse(
              state.height,
              Array.emptyByteArray,
              QueryCode.NotFound,
              s"Cannot parse query path: $path, must be in `sessionId/nonce` format"
            )
        )

      case Some(head) ⇒
        // It's a query for a particular response for a session and nonce
        state.get.map { st ⇒
          st.responses.find(_._1 == head) match {
            case Some((_, data)) ⇒
              QueryResponse(st.height, data, QueryCode.Ok, s"Responded for path $path")

            case _ ⇒
              // Is it pending or unknown?
              if (st.sessions.data.get(head.session).exists(_.nextNonce <= head.nonce))
                QueryResponse(
                  st.height,
                  Array.emptyByteArray,
                  QueryCode.Pending,
                  s"Transaction is not yet processed: $path"
                )
              else
                QueryResponse(
                  st.height,
                  Array.emptyByteArray,
                  QueryCode.NotFound,
                  s"No response found for path: $path"
                )
          }
        }
    }

  /**
   * Push incoming transaction to be processed on [[commit]].
   *
   * @param data Incoming transaction
   */
  def deliverTx(data: Array[Byte])(implicit log: Log[F]): F[TxResponse] =
    Tx.readTx[F](data)
      .semiflatMap { tx =>
        // TODO we have different logic in checkTx and deliverTx, as only in deliverTx tx might be dropped due to pending txs overflow
        // Update the state with a new tx
        state.modifyState(MachineState.addTx[Eval](tx).transform((s, c) => (s, (c, s.height)))).map {
          case (code, height) => (code, tx, height)
        }
      }
      .fold(TxResponse(TxCode.BAD, s"Cannot parse transaction header")) {
        case (code, tx, height) =>
          val infoMessage = code match {
            case TxCode.OK ⇒ s"Delivered"
            case TxCode.QueueDropped ⇒ s"Queue dropped due to being full with pending txs; next nonce should be 0"
            case TxCode.AlreadyProcessed ⇒ s"Tx is already processed, ignoring"
            case TxCode.BadNonce ⇒ s"Bad nonce: tx is out of order"
            case TxCode.BAD ⇒ s"Cannot parse transaction"
          }

          TxResponse(code, s"$infoMessage\n${tx.head}", Some(height))
      }

  /**
   * Check if transaction is well-formed: [[Tx.readTx()]] must return Some
   *
   * @param data Incoming transaction
   */
  def checkTx(data: Array[Byte])(implicit log: Log[F]): F[TxResponse] =
    Tx.readTx[F](data).value.flatMap {
      case Some(tx) ⇒
        state.get
          .map(
            state =>
              if (!state.sessions.data
                    .get(tx.head.session)
                    .exists(_.nextNonce > tx.head.nonce)) {
                TxResponse(TxCode.OK, s"Parsed transaction head\n${tx.head}", state.height.some)
              } else {
                TxResponse(TxCode.AlreadyProcessed, s"Transaction is already processed\n${tx.head}", state.height.some)
              }
          )
      case None ⇒
        Applicative[F].pure(TxResponse(TxCode.BAD, s"Cannot parse transaction header"))
    }
}

object StateService {

  /**
   * Build an empty StateService for the vm. App hash is empty!
   *
   * @param vm VM to invoke
   * @param receiptBusBackend To retrieve receipts and send vm hash
   * @tparam F Sync for Ref
   * @return Brand new StateService instance
   */
  def apply[F[_]: Effect: Log](
    vm: VmOperationInvoker[F],
    receiptBusBackend: ReceiptBusBackend[F]
  ): F[StateService[F]] =
    for {
      state ← Ref.of[F, MachineState](MachineState())
    } yield {
      val bva = Crypto.liftFunc[ByteVector, Array[Byte]](_.toArray)
      val abv = Crypto.liftFunc[Array[Byte], ByteVector](ByteVector(_))
      implicit val hasher: Crypto.Hasher[ByteVector, ByteVector] =
        bva.andThen[Array[Byte]](JdkCryptoHasher.Sha256).andThen(abv)

      new StateService[F](state, vm, receiptBusBackend)
    }
}
