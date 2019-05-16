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

import cats.effect.IO

import scala.language.higherKinds

/**
 * An interface to Kademlia-related calls for a remote node.
 *
 * @tparam C Type for contact data
 */
trait KademliaRpc[C] {

  /**
   * Ping the contact, get its actual Node status, or fail.
   */
  def ping(): IO[Node[C]]

  /**
   * Perform a local lookup for a key, return K closest known nodes.
   *
   * @param key Key to lookup
   */
  def lookup(key: Key, neighbors: Int): IO[Seq[Node[C]]]

  /**
   * Perform a local lookup for a key, return K closest known nodes, going away from the second key.
   *
   * @param key Key to lookup
   */
  def lookupAway(key: Key, moveAwayFrom: Key, neighbors: Int): IO[Seq[Node[C]]]
}
