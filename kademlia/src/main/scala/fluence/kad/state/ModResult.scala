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

import cats.MonoidK
import fluence.kad.protocol.{Key, Node}

/**
 * Result of state modification: some nodes are updated, others are removed
 *
 * @param updated Map of updated nodes
 * @param removed Set of removed keys
 * @tparam C Contact
 */
case class ModResult[C] private (updated: Map[Key, Node[C]], removed: Set[Key]) {

  /**
   * Update a node
   */
  def update(node: Node[C]): ModResult[C] =
    ModResult(
      updated + (node.key -> node),
      removed - node.key
    )

  /**
   * Remove a node, if it wasn't updated
   *
   * @param key Key to remove
   */
  def remove(key: Key): ModResult[C] =
    ModResult(
      updated,
      if (updated.contains(key)) removed else removed + key
    )

  /**
   * Do not remove a node
   */
  def keep(key: Key): ModResult[C] =
    copy(removed = removed - key)
}

object ModResult {
  def noop[C]: ModResult[C] = new ModResult[C](Map.empty, Set.empty)

  def updated[C](node: Node[C]): ModResult[C] =
    noop[C].update(node)

  def removed[C](key: Key): ModResult[C] =
    noop[C].remove(key)

  implicit object modResultMonoidK extends MonoidK[ModResult] {
    override def empty[A]: ModResult[A] = noop[A]

    override def combineK[A](x: ModResult[A], y: ModResult[A]): ModResult[A] =
      ModResult(
        x.updated ++ y.updated,
        // Do not remove a node if it was updated somehow
        (x.removed -- y.updated.keys) ++ (y.removed -- x.updated.keys)
      )
  }
}
