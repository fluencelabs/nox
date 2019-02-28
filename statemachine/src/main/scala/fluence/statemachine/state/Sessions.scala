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

case class Sessions(num: Int = 0, queue: Queue[String] = Queue.empty, data: Map[String, Session] = Map.empty)

object Sessions {

  def drop[F[_]: Monad](session: String, dropFromQueue: Boolean = true): StateT[F, Sessions, Boolean] =
    StateT.get[F, Sessions].flatMap {
      case Sessions(num, queue, data) if data.contains(session) ⇒
        StateT
          .set[F, Sessions](
            Sessions(
              num - 1,
              if (dropFromQueue) queue.filterNot(_ == session) else queue,
              data - session
            )
          )
          .map(_ ⇒ true)

      case _ ⇒
        StateT.pure(false)
    }

  def bound[F[_]: Monad](maxSessions: Int): StateT[F, Sessions, List[String]] =
    StateT.get[F, Sessions].flatMap {
      case Sessions(num, _, _) if num <= maxSessions ⇒ StateT.pure(Nil)
      case Sessions(num, queue, data) ⇒
        val (drop, keep) = queue.dequeue
        for {
          _ ← StateT.set(Sessions(num - 1, keep, data - drop))
          dropped ← bound(maxSessions)
        } yield drop :: dropped
    }

  def addTx[F[_]: Monad](tx: Tx, maxPending: Int): StateT[F, Sessions, Boolean] =
    StateT.get[F, Sessions].flatMap {
      case sessions @ Sessions(num, queue, data) ⇒
        import tx.head._

        data.get(session) match {
          case Some(Session(lastNonce, _)) if lastNonce <= nonce ⇒
            StateT.pure(false)

          case Some(Session(lastNonce, pendingTxs)) ⇒
            if (pendingTxs.size == maxPending)
              drop(session).map(!_)
            else
              StateT
                .set(
                  sessions.copy(data = data + (session -> Session(lastNonce, pendingTxs + (nonce -> tx.data))))
                )
                .map(_ ⇒ true)

          case None ⇒
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

  def formBlock[F[_]: Monad](sessions: List[String]): StateT[F, Sessions, List[Tx]] =
    sessions match {
      case s :: tail ⇒
        StateT.get[F, Sessions].flatMap {
          case Sessions(num, queue, data) ⇒
            data.get(s) match {
              case Some(session) ⇒
                for {
                  txs <- Session
                    .shake[F]
                    .map(_.map {
                      case (n, d) ⇒ Tx(Tx.Head(s, n), d)
                    })
                    .transformS[Sessions](_ ⇒ session, (ss, ses) ⇒ ss.copy(data = data + (s -> ses)))

                  others ← formBlock(tail)
                } yield txs ::: others

              case None ⇒
                formBlock(tail)
            }
        }

      case Nil ⇒
        StateT.pure(Nil)
    }
}
