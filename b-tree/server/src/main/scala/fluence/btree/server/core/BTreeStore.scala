/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

package fluence.btree.server.core

import scala.language.higherKinds

/**
  * BTree persistence store.
  *
  * @tparam F Some box for returning value
  * @tparam Id   The type of node id
  * @tparam Node The type of node
  */
trait BTreeStore[F[_], Id, Node] {

  /**
    * Returns next surrogate Id for storing new node.
    */
  def nextId(): Id

  /**
    * Gets stored node for specified id.
    * @param nodeId - id of stored the node.
    */
  def get(nodeId: Id): F[Node]

  /**
    * Store specified node with specified id.
    * Rewrite existing value if it's present.
    * @param nodeId - the specified node id to be inserted
    * @param node - the node associated with the specified node id
    */
  def put(nodeId: Id, node: Node): F[Unit]

  // todo: additional methods like 'remove' will be created on demand

}
