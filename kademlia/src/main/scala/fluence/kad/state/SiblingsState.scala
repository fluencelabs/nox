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

import cats.syntax.functor._
import cats.effect.Async
import fluence.kad.protocol.{Key, Node}
import fluence.log.Log

import scala.language.higherKinds

/**
 * Stateful Write operations for Siblings
 *
 * @tparam F Effect type
 * @tparam C Contact
 */
sealed trait SiblingsState[F[_], C] {

  def read: F[Siblings[C]]

  /**
   * Adds a node
   *
   * @param node Node to add
   * @return True if the node is in Siblings after update
   */
  def add(node: Node[C])(implicit log: Log[F]): F[ModResult[C]]

  /**
   * Removes a node
   *
   * @param key Node to remove
   * @return Optional node, if it was removed
   */
  def remove(key: Key)(implicit log: Log[F]): F[ModResult[C]]
}

object SiblingsState {

  /**
   * Builds asynchronous sibling ops with $maxSiblings nodes max.
   *
   * @param nodeKey      Siblings are sorted by distance to this nodeId
   * @param maxSize     Max number of closest siblings to store
   * @tparam C Node contacts type
   */
  private[state] def withRef[F[_]: Async, C](nodeKey: Key, maxSize: Int): F[SiblingsState[F, C]] =
    ReadableMVar
      .of[F, Siblings[C]](Siblings[C](nodeKey, maxSize))
      .map(
        rmv ⇒
          new SiblingsState[F, C] {
            override def read: F[Siblings[C]] = rmv.read

            override def add(node: Node[C])(implicit log: Log[F]): F[ModResult[C]] =
              rmv(Siblings.add(node))

            override def remove(key: Key)(implicit log: Log[F]): F[ModResult[C]] =
              rmv(Siblings.remove(key))
        }
      )
}
