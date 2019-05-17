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

package fluence.statemachine

import cats.effect.concurrent.Ref
import cats.effect.{Concurrent, Timer}
import cats.syntax.applicative._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.{Applicative, Monad}
import com.github.jtendermint.jabci.api.CodeType
import fluence.codec.PureCodec
import fluence.crypto.Crypto.Hasher
import fluence.crypto.hash.JdkCryptoHasher
import fluence.effects.Backoff
import fluence.effects.tendermint.block.history.Receipt
import fluence.statemachine.control.ControlSignals
import fluence.statemachine.state.AbciState
import scodec.bits.ByteVector
import slogging.LazyLogging

import scala.language.higherKinds

/**
 * Wraps all the state and logic required to perform ABCI logic.
 *
 * @param state See [[AbciState]]
 * @param vm Virtual machine invoker
 */
class AbciService[F[_]: Monad: Timer: Concurrent](
  state: Ref[F, AbciState],
  vm: VmOperationInvoker[F],
  controlSignals: ControlSignals[F]
)(implicit hasher: Hasher[ByteVector, ByteVector])
    extends LazyLogging {

  import AbciService._

  def stateHeight = state.get.map(_.height)

  /**
   * Take all the transactions we're able to process, and pass them to VM one by one.
   *
   * @return App (VM) Hash
   */
  def commit: F[ByteVector] =
    for {
      // Get current state
      s ← state.get
      // Form a block: take ordered txs from AbciState
      sTxs ← AbciState.formBlock[F].run(s)

      // Process txs one by one
      st ← Monad[F].tailRecM[(AbciState, List[Tx]), AbciState](sTxs) {
        case (st, tx :: txs) ⇒
          // Invoke
          vm.invoke(tx.data.value)
            // Save the tx response to AbciState
            .semiflatMap(value ⇒ AbciState.putResponse[F](tx.head, value).map(_ ⇒ txs).run(st).map(Left(_)))
            .leftMap(err ⇒ logger.error(s"VM invoke failed: $err for tx: $tx"))
            .getOrElse(Right(st)) // TODO do not ignore vm error

        case (st, Nil) ⇒
          Applicative[F].pure(Right(st))
      }

      // Get the VM hash
      vmHash ← vm
        .vmStateHash()
        .leftMap(err ⇒ logger.error(s"VM is unable to compute state hash: $err"))
        .getOrElse(ByteVector.empty) // TODO do not ignore vm error

      receipt <- controlSignals.receipt
      appHash <- receipt.fold(vmHash.pure[F])(
        r =>
          hasher(vmHash ++ r.bytes())
            .leftMap(err => logger.error(s"Error on hashing vmHash + receipt: $err"))
            .getOrElse(vmHash) // TODO: that's awful; don't ignore errors
      )

      // Push hash to AbciState, increment block number
      newState ← AbciState.setAppHash(appHash).runS(st)

      // Store updated state in the Ref (the changes were transient for readers before this step)
      _ ← state.set(newState)
    } yield appHash

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
              Codes.NotFound,
              s"Cannot parse query path: $path, must be in `sessionId/nonce` format"
          )
        )

      case Some(head) ⇒
        // It's a query for a particular response for a session and nonce
        state.get.map { st ⇒
          st.responses.find(_._1 == head) match {
            case Some((_, data)) ⇒
              QueryResponse(st.height, data, Codes.Ok, s"Responded for path $path")

            case _ ⇒
              // Is it pending or unknown?
              if (st.sessions.data.get(head.session).exists(_.nextNonce <= head.nonce))
                QueryResponse(
                  st.height,
                  Array.emptyByteArray,
                  Codes.Pending,
                  s"Transaction is not yet processed: $path"
                )
              else
                QueryResponse(
                  st.height,
                  Array.emptyByteArray,
                  Codes.NotFound,
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
  def deliverTx(data: Array[Byte]): F[TxResponse] =
    Tx.readTx(data) match {
      case Some(tx) ⇒
        // TODO we have different logic in checkTx and deliverTx, as only in deliverTx tx might be dropped due to pending txs overflow
        state
        // Update the state with a new tx
          .modifyState(AbciState.addTx(tx))
          .map {
            case true ⇒ TxResponse(CodeType.OK, s"Delivered\n${tx.head}")
            case false ⇒ TxResponse(CodeType.BadNonce, s"Dropped\n${tx.head}")
          }
      case None ⇒
        Applicative[F].pure(TxResponse(CodeType.BAD, s"Cannot parse transaction header"))
    }

  /**
   * Check if transaction is well-formed: [[Tx.readTx()]] must return Some
   *
   * @param data Incoming transaction
   */
  def checkTx(data: Array[Byte]): F[TxResponse] =
    Tx.readTx(data) match {
      case Some(tx) ⇒
        state.get
          .map(
            !_.sessions.data
              .get(tx.head.session)
              .exists(_.nextNonce > tx.head.nonce)
          )
          .map {
            case true ⇒
              // Session is unknown, or nonce is valid
              TxResponse(CodeType.OK, s"Parsed transaction head: ${tx.head}")
            case false ⇒
              // Invalid nonce -- misorder
              TxResponse(CodeType.BadNonce, s"Misordered\n${tx.head}")
          }
      case None ⇒
        Applicative[F].pure(TxResponse(CodeType.BAD, s"Cannot parse transaction header"))
    }
}

object AbciService {

  object Codes {
    val Ok: Int = 0
    val CannotParseHeader: Int = 1
    val Dropped: Int = 2
    val NotFound: Int = 3
    val Pending: Int = 4
  }

  /**
   * A structure for aggregating data specific to building `Query` ABCI method response.
   *
   * @param height height corresponding to state for which result given
   * @param result requested result, if found
   * @param code response code
   * @param info response message
   */
  case class QueryResponse(height: Long, result: Array[Byte], code: Int, info: String)

  /**
   * A structure for aggregating data specific to building `CheckTx`/`DeliverTx` ABCI response.
   *
   * @param code response code
   * @param info response message
   */
  case class TxResponse(code: Int, info: String)

  /**
   * Build an empty AbciService for the vm. App hash is empty!
   *
   * @param vm VM to invoke
   * @tparam F Sync for Ref
   * @return Brand new AbciService instance
   */
  def apply[F[_]: Concurrent: Timer](vm: VmOperationInvoker[F], controlSignals: ControlSignals[F]): F[AbciService[F]] = {
    import cats.syntax.applicative._
    import cats.syntax.compose._

    for {
      state ← Ref.of[F, AbciState](AbciState())
      fiber <- Concurrent[F].start(Option.empty[Receipt].pure[F])
    } yield {
      implicit val hasher: Hasher[Array[Byte], Array[Byte]] =
        PureCodec[ByteVector, Array[Byte]].andThen(JdkCryptoHasher.Sha256)
      new AbciService[F](state, vm, controlSignals)
    }
  }

}
