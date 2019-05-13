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

import cats.Eval
import cats.syntax.functor._
import cats.effect.Sync
import cats.effect.concurrent.Ref
import fluence.kad.protocol.{Key, Node}

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

  /**
   * Builds asynchronous sibling ops with $maxSiblings nodes max.
   * Note that it is safe to use Ref, effectively blocking on state changes, as there's no I/O delays, see [[Bucket.update]]
   *
   * @param nodeKey      Siblings are sorted by distance to this nodeId
   * @param maxSize     Max number of closest siblings to store
   * @tparam C Node contacts type
   */
  private[state] def withRef[F[_]: Sync, C](nodeKey: Key, maxSize: Int): F[SiblingsState[F, C]] =
    Ref
      .of[F, Siblings[C]](Siblings[C](nodeKey, maxSize))
      .map(
        ref ⇒
          new SiblingsState[F, C] {
            override def read: F[Siblings[C]] = ref.get

            override def add(node: Node[C]): F[ModResult[C]] =
              ref.modifyState(Siblings.add[Eval, C](node))

            override def remove(key: Key): F[ModResult[C]] =
              ref.modifyState(Siblings.remove[Eval, C](key))
        }
      )
}
