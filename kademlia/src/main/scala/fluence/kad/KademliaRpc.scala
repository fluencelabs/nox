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

package fluence.kad

import scala.language.higherKinds

/**
 * An interface to Kademlia-related calls for a remote node
 * @tparam F An effect, with MonadError
 * @tparam C Type for contact data
 */
trait KademliaRpc[F[_], C] {
  /**
   * Ping the contact, get its actual Node status, or fail
   */
  def ping(): F[Node[C]]

  /**
   * Perform a local lookup for a key, return K closest known nodes
   * @param key Key to lookup
   */
  def lookup(key: Key, numberOfNodes: Int): F[Seq[Node[C]]]

  /**
   * Perform a local lookup for a key, return K closest known nodes, going away from the second key
   * @param key Key to lookup
   */
  def lookupAway(key: Key, moveAwayFrom: Key, numberOfNodes: Int): F[Seq[Node[C]]]

  /**
   * Perform an iterative lookup for a key, return K closest known nodes
   * @param key Key to lookup
   */
  // TODO: it's used only in [[RoutingTable.WriteOps.join()]]; gives the way to perform a big number of requests
  // on distant node with RPC call; could be unsafe to expose with RPC
  def lookupIterative(key: Key, numberOfNodes: Int): F[Seq[Node[C]]]

}
