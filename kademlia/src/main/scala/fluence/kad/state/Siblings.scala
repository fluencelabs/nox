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

import cats.{Applicative, Monad, Show}
import cats.data.StateT
import cats.syntax.eq._
import cats.syntax.functor._
import cats.syntax.apply._
import cats.syntax.applicative._
import fluence.kad.protocol.{Key, Node}
import fluence.log.Log

import scala.collection.SortedSet
import scala.language.higherKinds

/**
 * List of the closest known nodes for the current one
 *
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

  def remove(key: Key): Siblings[C] =
    copy(nodes.filterNot(_.key === key))
}

object Siblings {
  implicit def show[C](implicit ks: Show[Key]): Show[Siblings[C]] =
    s ⇒ s.nodes.toSeq.map(_.key).map(ks.show).mkString(s"\nSiblings: ${s.size}\n\t", "\n\t", "")

  /**
   * Add a node to the siblings, or drop it if all known siblings are closer then the given node
   *
   * @param node Node to add to Siblings
   * @tparam F Effect
   * @tparam C Contact
   * @return State modification
   */
  def add[F[_]: Monad: Log, C](node: Node[C]): StateT[F, Siblings[C], ModResult[C]] =
    StateT.get[F, Siblings[C]].flatMap { st ⇒
      val (keep, drop) = (st.nodes + node).splitAt(st.maxSize)

      StateT
        .set(st.copy(keep))
        .flatMapF(
          _ ⇒
            if (drop.toList == List(node)) ModResult.noop[C].pure[F]
            else
              Log[F].trace(s"Sibling updated ${node.key}") *>
                Monad[F].tailRecM(drop.map(_.key).toList -> ModResult.updated(node)) {
                  case (Nil, mr) ⇒ Applicative[F].pure(Right(mr))
                  case (d :: tail, mr) ⇒ Log[F].trace(s"Pushed $d away by siblings") as Left(tail -> mr.remove(d))
              }
        )
    }

  /**
   * Remove a node from siblings, if it's present
   *
   * @param key Node's key to remove
   * @tparam F Effect
   * @tparam C Contact
   * @return State modification
   */
  def remove[F[_]: Monad: Log, C](key: Key): StateT[F, Siblings[C], ModResult[C]] =
    StateT.get[F, Siblings[C]].flatMap {
      case st if st.contains(key) ⇒
        StateT
          .set(
            st.copy(st.nodes.filterNot(_.key === key))
          )
          .flatMapF(_ ⇒ Log[F].trace(s"Remove $key from siblings") as ModResult.removed(key))
      case _ ⇒
        StateT.pure(ModResult.noop)
    }

  /**
   * Builds a Siblings instance with ordering relative to nodeKey
   *
   * @param nodeKey Current node's Kademlia key
   * @param maxSize Maximum number of sibling nodes to keep
   * @tparam C Contact info
   */
  def apply[C](nodeKey: Key, maxSize: Int): Siblings[C] =
    new Siblings[C](
      SortedSet.empty(Node.relativeOrdering(nodeKey)),
      maxSize
    )

}
