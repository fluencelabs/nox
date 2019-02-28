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

import cats.effect.Sync
import cats.{Applicative, Monad}
import cats.syntax.functor._
import cats.syntax.flatMap._
import cats.effect.concurrent.Ref
import fluence.statemachine.state.AbciState
import scodec.bits.ByteVector
import slogging.LazyLogging

import scala.language.higherKinds

class AbciService[F[_]: Monad](
  state: Ref[F, AbciState],
  vm: VmOperationInvoker[F]
) extends LazyLogging {

  import AbciService._

  // returns app hash
  def commit: F[ByteVector] =
    for {
      s ← state.get
      sTxs ← AbciState.formBlock[F].run(s)

      st ← Monad[F].tailRecM[(AbciState, List[Tx]), AbciState](sTxs) {
        case (st, tx :: txs) ⇒
          vm.invoke(tx.data.value)
            .semiflatMap(value ⇒ AbciState.putResponse[F](tx.head, value).map(_ ⇒ txs).run(st).map(Left(_)))
            .leftMap(err ⇒ logger.error(s"VM invoke failed: $err for tx: $tx"))
            .getOrElse(Right(st)) // TODO do not ignore vm error

        case (st, Nil) ⇒
          Applicative[F].pure(Right(st))
      }

      hash ← vm
        .vmStateHash()
        .leftMap(err ⇒ logger.error(s"VM is unable to compute state hash: $err"))
        .getOrElse(ByteVector.empty) // TODO do not ignore vm error

      newState ← AbciState.setAppHash(hash).runS(st)

      _ ← state.set(newState)
    } yield hash

  def query(path: String): F[QueryResponse] =
    Tx.readHead(path) match {
      case None ⇒
        state.get.map(
          state ⇒
            state.sessions.data.get(path) match {
              case Some(ses) ⇒
                QueryResponse(
                  state.height,
                  s"nextNonce:${ses.nextNonce}".getBytes(),
                  0,
                  s"Session $path is found"
                )

              case None ⇒
                QueryResponse(
                  state.height,
                  Array.emptyByteArray,
                  1,
                  s"Cannot parse query path: $path, must be in `session-nonce` format"
                )
          }
        )

      case Some(head) ⇒
        state.get.map(s ⇒ s.responses.find(_._1 == head) -> s.height).map {
          case (Some((_, data)), h) ⇒
            QueryResponse(h, data, 0, s"Responded for path $path")
          case (_, h) ⇒
            QueryResponse(h, Array.emptyByteArray, 2, s"No response found for path: $path")
        }
    }

  def deliverTx(data: Array[Byte]): F[TxResponse] =
    Tx.readTx(data) match {
      case Some(tx) ⇒
        state.modifyState(AbciState.addTx(tx)).map(_ ⇒ TxResponse(0, s"Transaction delivered: ${tx.head}"))
      case None ⇒ Applicative[F].pure(TxResponse(1, s"Cannot parse transaction header"))
    }

  def checkTx(data: Array[Byte]): F[TxResponse] =
    Tx.readTx(data) match {
      case Some(tx) ⇒ Applicative[F].pure(TxResponse(0, s"Parsed transaction head: ${tx.head}"))
      case None ⇒ Applicative[F].pure(TxResponse(1, s"Cannot parse transaction header"))
    }
}

object AbciService {

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

  def apply[F[_]: Sync](vm: VmOperationInvoker[F]): F[AbciService[F]] =
    for {
      state ← Ref.of[F, AbciState](AbciState())
    } yield new AbciService[F](state, vm)

}
