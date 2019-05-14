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

package fluence.kad.protocol

import cats.{Order, Show}

/**
 * Kademlia's Node representation.
 *
 * @param key Key
 * @param contact Description on how to contact the node over network
 * @tparam C Contact info
 */
case class Node[C](
  key: Key,
  contact: C
)

object Node {
  implicit def show[C](implicit ks: Show[Key], cs: Show[C]): Show[Node[C]] =
    n ⇒ s"Node(${ks.show(n.key)}, ${cs.show(n.contact)})"

  /**
   * Builds order by distance relative to target node.
   *
   * @param key Node to calculate distance against
   * @tparam C Contact type
   */
  def relativeOrder[C](key: Key): Order[Node[C]] = new Order[Node[C]] {
    private val order = Key.relativeOrder(key)

    override def compare(x: Node[C], y: Node[C]): Int = order.compare(x.key, y.key)
  }

  def relativeOrdering[C](key: Key): Ordering[Node[C]] =
    relativeOrder(key).compare(_, _)
}
