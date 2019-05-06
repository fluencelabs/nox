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

import cats.Monad
import cats.data.StateT
import cats.syntax.functor._
import cats.syntax.flatMap._
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

  protected def run[T](mod: StateT[F, Siblings[C], T]): F[T]

  /**
   * Adds a node
   *
   * @param node Node to add
   * @param F Monad
   * @return True if node is in Siblings after update
   */
  def add(node: Node[C])(implicit F: Monad[F]): F[Boolean] =
    for {
      _ ← run(StateT.modify(_.add(node)))
      siblings ← read
    } yield siblings.contains(node.key)

  /**
   * Removes a node
   *
   * @param key Node to remove
   * @param F Monad
   * @return Optional node, if it was removed
   */
  def remove(key: Key)(implicit F: Monad[F]): F[Option[Node[C]]] =
    for {
      siblings ← read
      nodeOpt = siblings.find(key)
      _ ← if (nodeOpt.isDefined) run(StateT.modify(_.remove(key)))
      else F.pure(())
    } yield nodeOpt
}
