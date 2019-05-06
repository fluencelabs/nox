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

package fluence.kad

import cats.Show
import cats.syntax.eq._
import fluence.kad.protocol.{Key, Node}

import scala.collection.SortedSet
import scala.language.higherKinds

/**
 * List of the closest known nodes for the current one
 * @param nodes Nodes, ordered by distance
 * @param maxSize Maximum number of sibling nodes to keep
 * @tparam C Contact type
 */
case class Siblings[C] private (nodes: SortedSet[Node[C]], maxSize: Int) {

  lazy val isFull: Boolean = nodes.size >= maxSize

  lazy val size: Int = nodes.size

  def isEmpty: Boolean = nodes.isEmpty

  def nonEmpty: Boolean = nodes.nonEmpty

  def find(key: Key): Option[Node[C]] = nodes.find(_.key === key)

  def contains(key: Key): Boolean = nodes.exists(_.key === key)

  def add(node: Node[C]): Siblings[C] =
    copy((nodes + node).take(maxSize))

  def remove(key: Key): Siblings[C] =
    copy(nodes.filterNot(_.key === key))
}

object Siblings {
  implicit def show[C](implicit ks: Show[Key]): Show[Siblings[C]] =
    s ⇒ s.nodes.toSeq.map(_.key).map(ks.show).mkString(s"\nSiblings: ${s.size}\n\t", "\n\t", "")

  /**
   * Builds a Siblings instance with ordering relative to nodeId
   * @param nodeId Current node's id
   * @param maxSize Maximum number of sibling nodes to keep
   * @tparam C Contact info
   */
  def apply[C](nodeId: Key, maxSize: Int): Siblings[C] =
    new Siblings[C](
      SortedSet.empty(Node.relativeOrdering(nodeId)),
      maxSize
    )

}
