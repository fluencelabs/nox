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

import cats.data.StateT
import cats.{Applicative, Functor, Monad}
import fluence.statemachine.Tx
import scodec.bits.ByteVector

import scala.collection.immutable.Queue
import scala.language.higherKinds

case class AbciState(
  height: Long = 0,
  appHash: ByteVector = ByteVector.empty,
  blockSessions: Set[String] = Set.empty,
  responses: Queue[(Tx.Head, Array[Byte])] = Queue.empty,
  sessions: Sessions = Sessions()
)

object AbciState {

  private implicit class LensSessions[F[_]: Functor, T](sessionsState: StateT[F, Sessions, T]) {

    def toAbciState: StateT[F, AbciState, T] =
      sessionsState.transformS(_.sessions, (st, sess) ⇒ st.copy(sessions = sess))
  }

  def addTx[F[_]: Monad](tx: Tx, maxSessions: Int = 128, maxPendingTxs: Int = 24): StateT[F, AbciState, Boolean] =
    for {
      added ← Sessions.addTx(tx, maxPendingTxs).toAbciState
      _ ← Sessions.bound(maxSessions).toAbciState
      _ ← StateT.modify[F, AbciState](
        st ⇒ st.copy(blockSessions = if (added) st.blockSessions + tx.head.session else st.blockSessions)
      )
    } yield added

  def boundResponses[F[_]: Monad](limit: Int): StateT[F, AbciState, List[(Tx.Head, Array[Byte])]] =
    StateT.get[F, AbciState].flatMap {
      case st if st.responses.size > limit ⇒
        val (drop, keep) = st.responses.dequeue
        for {
          _ ← StateT.set(st.copy(responses = keep))
          others ← boundResponses(limit)
        } yield drop :: others

      case _ ⇒
        StateT.pure(Nil)
    }

  def putResponse[F[_]: Monad](head: Tx.Head, data: Array[Byte], resultsLimit: Int = 64): StateT[F, AbciState, Unit] =
    for {
      _ ← StateT.modify[F, AbciState](s ⇒ s.copy(responses = s.responses.enqueue(head -> data)))
      _ ← boundResponses(resultsLimit)
    } yield ()

  def setAppHash[F[_]: Applicative](hash: ByteVector): StateT[F, AbciState, Unit] =
    StateT.modify(s ⇒ s.copy(height = s.height + 1, appHash = hash))

  def formBlock[F[_]: Monad]: StateT[F, AbciState, List[Tx]] =
    for {
      state ← StateT.get[F, AbciState]
      txs ← Sessions.formBlock[F](state.blockSessions.toList).toAbciState
      _ ← StateT.modify[F, AbciState](s ⇒ s.copy(blockSessions = Set.empty))
    } yield txs
}
