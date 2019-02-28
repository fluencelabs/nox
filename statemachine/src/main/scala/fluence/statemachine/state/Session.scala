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
import cats.Monad
import cats.data.StateT
import fluence.statemachine.Tx

import scala.collection.immutable.TreeMap
import scala.language.higherKinds

/**
 * Single session data, required for correct ordering.
 *
 * @param nextNonce Next nonce (order index), starting with 0, incremented with no gaps
 * @param pendingTxs Ordered map of pending transactions
 */
case class Session(nextNonce: Long, pendingTxs: TreeMap[Long, Tx.Data])

object Session {

  /**
   * Takes all the txs from the beginning, starting with nextNonce, and drops them away.
   *
   * @return List of txs to handle in this block
   */
  def commit[F[_]: Monad]: StateT[F, Session, List[(Long, Tx.Data)]] =
    StateT.get[F, Session].flatMap {
      case Session(nextNonce, pendingTxs) ⇒
        pendingTxs.get(nextNonce) match {
          case Some(data) ⇒
            for {
              _ <- StateT.set(Session(nextNonce + 1, pendingTxs - nextNonce))
              others ← commit[F]
            } yield (nextNonce, data) :: others

          case None ⇒
            StateT.pure(Nil)
        }
    }
}
