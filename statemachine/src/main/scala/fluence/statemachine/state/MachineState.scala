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
import fluence.bp.tx.{Tx, TxCode}
import fluence.statemachine.api.data.StateHash
import scodec.bits.ByteVector

import scala.collection.immutable.Queue
import scala.language.higherKinds

/**
 * All the state for State Machine: collects the block, stores responses and sessions.
 *
 * @param height Number of commits performed on this MachineState instance
 * @param appHash Last known app hash
 * @param blockSessions Session IDs affected since the last commit
 * @param responses Queue of responses for handled txs
 * @param sessions Sessions data
 */
case class MachineState(
  height: Long = 0,
  appHash: ByteVector = ByteVector.empty,
  blockSessions: Set[String] = Set.empty,
  responses: Queue[(Tx.Head, Array[Byte])] = Queue.empty,
  sessions: Sessions = Sessions()
) {
  def stateHash: StateHash = StateHash(height, appHash)
}

object MachineState {

  private implicit class LensSessions[F[_]: Functor, T](sessionsState: StateT[F, Sessions, T]) {

    def toMachineState: StateT[F, MachineState, T] =
      sessionsState.transformS(_.sessions, (st, sess) ⇒ st.copy(sessions = sess))
  }

  /**
   * Add transaction, which is to be handled later.
   *
   * @param tx Incoming transaction
   * @param maxSessions The upper bound for the number of stored sessions TODO: move to config
   * @param maxPendingTxs The upper bound for the number of cached pending txs for a single session TODO: move to config
   * @return Whether tx was stored in AbciState or is ignored
   */
  def addTx[F[_]: Monad](
    tx: Tx,
    maxSessions: Int = 128,
    maxPendingTxs: Int = 256
  ): StateT[F, MachineState, TxCode.Value] =
    for {
      // Add tx to sessions
      code ← Sessions.addTx(tx, maxPendingTxs).toMachineState
      // Drop sessions
      _ ← Sessions.bound(maxSessions).toMachineState
      // If this tx was added, keep it in blockSessions
      _ ← StateT.modify[F, MachineState](
        st ⇒ st.copy(blockSessions = if (code == TxCode.OK) st.blockSessions + tx.head.session else st.blockSessions)
      )
    } yield code

  /**
   * Apply the bound for responses -- drop the oldest ones.
   *
   * @param limit Max number of responses to store
   * @return List of dropped responses
   */
  def boundResponses[F[_]: Monad](limit: Int): StateT[F, MachineState, List[(Tx.Head, Array[Byte])]] =
    StateT.get[F, MachineState].flatMap {
      case st if st.responses.size > limit ⇒
        // Can dequeue safely
        val (drop, keep) = st.responses.dequeue
        for {
          // Drop one
          _ ← StateT.set(st.copy(responses = keep))
          // Drop more. TODO is it effective?
          others ← boundResponses(limit)
        } yield drop :: others

      case _ ⇒
        // No need to drop anything
        StateT.pure(Nil)
    }

  /**
   * Put response to state cache.
   *
   * @param head Tx head, used to address the response
   * @param data Response data
   * @param resultsLimit How many results we are allowed to keep in cache TODO: move to config
   */
  def putResponse[F[_]: Monad](
    head: Tx.Head,
    data: Array[Byte],
    resultsLimit: Int = 512
  ): StateT[F, MachineState, Unit] =
    for {
      _ ← StateT.modify[F, MachineState](s ⇒ s.copy(responses = s.responses.enqueue(head -> data)))
      _ ← boundResponses(resultsLimit)
    } yield ()

  /**
   * Set app hash once the block is processed, increment height by 1.
   *
   * @param hash App hash
   */
  def setAppHash[F[_]: Applicative](hash: ByteVector): StateT[F, MachineState, Unit] =
    StateT.modify(s ⇒ s.copy(height = s.height + 1, appHash = hash))

  /**
   * Get list of transactions to be handled in this block, in order.
   *
   * @return List of Txs, they're already removed from caches
   */
  def formBlock[F[_]: Monad]: StateT[F, MachineState, List[Tx]] =
    for {
      state ← StateT.get[F, MachineState]
      txs ← Sessions.commit[F](state.blockSessions.toList).toMachineState
      _ ← StateT.modify[F, MachineState](s ⇒ s.copy(blockSessions = Set.empty))
      _ ← Sessions.refresh[F](state.blockSessions).toMachineState
    } yield txs
}
