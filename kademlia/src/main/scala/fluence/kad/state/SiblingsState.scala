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

package fluence.kad.state

import cats.Monad
import cats.syntax.functor._
import cats.effect.Async
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
  def add(node: Node[C]): F[ModResult[C]]

  /**
   * Removes a node
   *
   * @param key Node to remove
   * @return Optional node, if it was removed
   */
  def remove(key: Key): F[ModResult[C]]
}

object SiblingsState {

  def forMVar[F[_]: Monad, C](state: ReadableMVar[F, Siblings[C]]): SiblingsState[F, C] =
    new SiblingsState[F, C] {
      override val read: F[Siblings[C]] = state.read

      override def add(node: Node[C]): F[ModResult[C]] =
        state.run(Siblings.add(node))

      override def remove(key: Key): F[ModResult[C]] =
        state.run(Siblings.remove(key))
    }

  /**
   * Builds asynchronous sibling ops with $maxSiblings nodes max.
   *
   * @param nodeId      Siblings are sorted by distance to this nodeId
   * @param maxSize     Max number of closest siblings to store
   * @tparam C Node contacts type
   */
  def withMVar[F[_]: Async, C](nodeId: Key, maxSize: Int): F[SiblingsState[F, C]] =
    ReadableMVar.of(Siblings[C](nodeId, maxSize)).map(forMVar[F, C])
}
