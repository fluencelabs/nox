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

package fluence.kad.core

import cats.{~>, Monad}
import cats.data.StateT
import cats.effect.Concurrent
import cats.syntax.functor._
import cats.syntax.apply._
import cats.syntax.flatMap._
import cats.syntax.applicative._
import fluence.effects.kvstore.KVStore
import fluence.kad.protocol.{Key, Node}

import scala.language.higherKinds

/**
 * Stateful Write operations for Siblings
 *
 * @tparam F Effect type
 * @tparam C Contact
 */
trait SiblingsState[F[_], C] {

  def read: F[Siblings[C]]

  /**
   * Adds a node
   *
   * @param node Node to add
   * @return True if the node is in Siblings after update
   */
  def add(node: Node[C]): F[Boolean]

  /**
   * Removes a node
   *
   * @param key Node to remove
   * @return Optional node, if it was removed
   */
  def remove(key: Key): F[Option[Node[C]]]
}

object SiblingsState {

  def liftState[F[_]: Monad, C](run: StateT[F, Siblings[C], ?] ~> F, readFn: F[Siblings[C]]): SiblingsState[F, C] =
    new SiblingsState[F, C] {
      override val read: F[Siblings[C]] = readFn

      override def add(node: Node[C]): F[Boolean] =
        for {
          _ ← run(StateT.modify(_.add(node)))
          siblings ← read
        } yield siblings.contains(node.key)

      override def remove(key: Key): F[Option[Node[C]]] =
        for {
          siblings ← read
          nodeOpt = siblings.find(key)
          _ ← if (nodeOpt.isDefined) run(StateT.modify(_.remove(key)))
          else ().pure[F]
        } yield nodeOpt
    }

  def stored[F[_]: Concurrent, C](state: SiblingsState[F, C], kvstore: KVStore[F, Key, Node[C]]): SiblingsState[F, C] =
    new SiblingsState[F, C] {
      override val read: F[Siblings[C]] = state.read

      /**
       * Adds a node
       *
       * @param node Node to add
       * @return True if the node is in Siblings after update
       */
      override def add(node: Node[C]): F[Boolean] =
        state.add(node) <* Concurrent[F].start(kvstore.put(node.key, node).value)

      /**
       * Removes a node
       *
       * @param key Node to remove
       * @return Optional node, if it was removed
       */
      override def remove(key: Key): F[Option[Node[C]]] =
        state.remove(key) <* Concurrent[F].start(kvstore.remove(key).value)
    }
}
