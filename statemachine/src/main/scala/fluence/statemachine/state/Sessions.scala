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

import scala.collection.immutable.{Queue, TreeMap}
import scala.language.higherKinds

/**
 * Cached sessions data.
 *
 * @param num Number of sessions, used to avoid O(n) length counts
 * @param queue Queue of session ids, last used on top, least used in tail
 * @param data Sessions data
 */
case class Sessions(num: Int = 0, queue: Queue[String] = Queue.empty, data: Map[String, Session] = Map.empty)

object Sessions {

  /**
   * Drop a session.
   *
   * @param session Session ID
   * @param dropFromQueue Whether to drop it from queue, or only from data
   * @return Whether the session was dropped or not
   */
  def drop[F[_]: Monad](session: String, dropFromQueue: Boolean = true): StateT[F, Sessions, Boolean] =
    StateT.get[F, Sessions].flatMap {
      case Sessions(num, queue, data) if data.contains(session) ⇒
        // Session found, so dropping is an effect
        StateT
          .set[F, Sessions](
            Sessions(
              // Decrement
              num - 1,
              // Dequeue
              if (dropFromQueue) queue.filterNot(_ == session) else queue,
              // Remove
              data - session
            )
          )
          .map(_ ⇒ true)

      case _ ⇒
        // Session is not present in Sessions
        StateT.pure(false)
    }

  /**
   * Bound the sessions with the given number. Those beyond the limit are dequeued and removed.
   *
   * @param maxSessions How many sessions is allowed to store.
   * @return List of dropped sessions
   */
  def bound[F[_]: Monad](maxSessions: Int): StateT[F, Sessions, List[String]] =
    StateT.get[F, Sessions].flatMap {
      case Sessions(num, _, _) if num <= maxSessions || maxSessions == 0 ⇒
        // No need to drop anything
        StateT.pure(Nil)

      case Sessions(num, queue, data) ⇒
        // Can dequeue safely
        val (drop, keep) = queue.dequeue
        for {
          // Drop one session -- the next in the queue
          _ ← StateT.set(Sessions(num - 1, keep, data - drop))
          // If we have some more, check if we need to drop them
          dropped ← bound(maxSessions)
        } yield drop :: dropped
    }

  /**
   * Add a transaction to the Session cache. Drop Session if it's cache is beyond the limit.
   *
   * @param tx Incoming transaction to add
   * @param maxPending Number of max pending transactions per one session
   * @return Whether this Tx was stored or not
   */
  def addTx[F[_]: Monad](tx: Tx, maxPending: Int): StateT[F, Sessions, Boolean] =
    StateT.get[F, Sessions].flatMap {
      case sessions @ Sessions(num, queue, data) ⇒
        import tx.head._

        // Find cache for this session
        data.get(session) match {
          case Some(Session(lastNonce, _)) if lastNonce > nonce ⇒
            // If this tx is already processed, ignore it
            StateT.pure(false)

          case Some(Session(lastNonce, pendingTxs)) ⇒
            // If this session is too big already, drop it
            if (pendingTxs.size >= maxPending)
              drop(session).map(_ ⇒ false)
            else
              // Add a pending tx to existing session
              StateT
                .set(
                  sessions.copy(data = data + (session -> Session(lastNonce, pendingTxs + (nonce -> tx.data))))
                )
                .map(_ ⇒ true)

          case None ⇒
            // Create a new session with a single pending tx
            StateT
              .set(
                Sessions(
                  num + 1,
                  queue.enqueue(tx.head.session),
                  data + (tx.head.session -> Session(0, TreeMap(tx.head.nonce -> tx.data)))
                )
              )
              .map(_ ⇒ true)
        }
    }

  /**
   * Prepare a block commit -- take all the transactions in order to process.
   *
   * @param sessions Sessions to look at -- those affected in this block
   * @return List of transactions to be processed in order
   */
  def commit[F[_]: Monad](sessions: List[String]): StateT[F, Sessions, List[Tx]] =
    sessions match {
      case s :: tail ⇒
        // Get the session
        StateT.get[F, Sessions].flatMap {
          case Sessions(_, queue, data) ⇒
            data.get(s) match {
              case Some(session) ⇒
                for {
                  // Make transactions from session
                  txs <- Session
                    .commit[F]
                    .map(_.map {
                      case (n, d) ⇒ Tx(Tx.Head(s, n), d)
                    })
                    .transformS[Sessions](_ ⇒ session, (ss, ses) ⇒ ss.copy(data = data + (s -> ses)))

                  // Collect all the next sessions
                  others ← commit(tail)
                } yield txs ::: others

              case None ⇒
                // This session is absent, look for others
                commit(tail)
            }
        }

      case Nil ⇒
        // No sessions touched -- do nothing
        StateT.pure(Nil)
    }

  /**
   * Refresh sessions -- mark them as recently used
   *
   * @param sessions Set of sessions to touch
   */
  def refresh[F[_]: Monad](sessions: Set[String]): StateT[F, Sessions, Unit] =
    StateT.modify(
      s ⇒
        s.copy(
          queue = s.queue.filterNot(sessions).enqueue(sessions)
      )
    )
}
