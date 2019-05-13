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

import cats.effect.{Effect, IO}
import cats.effect.syntax.effect._
import fluence.kad.routing.LocalRouting
import fluence.kad.protocol.{KademliaRpc, Key, Node}

import scala.language.higherKinds

/**
 * Handler for incoming Kademlia RPC requests
 *
 * @param loopbackContact This node's contact
 * @param localRouting Local routing table
 * @tparam F Effect, to be converted to IO
 * @tparam C Type for contact data
 */
private[kad] class LocalRpc[F[_]: Effect, C](loopbackContact: F[Node[C]], localRouting: LocalRouting[F, C])
    extends KademliaRpc[C] with slogging.LazyLogging {
  import localRouting.nodeKey

  /**
   * Ping the contact, get its actual Node status, or fail.
   */
  override def ping(): IO[Node[C]] = {
    logger.trace(s"HandleRPC($nodeKey): ping")
    loopbackContact.toIO
  }

  /**
   * Perform a local lookup for a key, return K closest known nodes.
   *
   * @param key Key to lookup
   */
  override def lookup(key: Key, numberOfNodes: Int): IO[Seq[Node[C]]] =
    IO.suspend {
      logger.trace(s"HandleRPC($nodeKey): lookup($key, $numberOfNodes)")

      localRouting.lookup(key, numberOfNodes).toIO
    }

  /**
   * Perform a local lookup for a key, return K closest known nodes, going away from the second key.
   *
   * @param key Key to lookup
   */
  override def lookupAway(key: Key, moveAwayFrom: Key, numberOfNodes: Int): IO[Seq[Node[C]]] =
    IO.suspend {
      logger.trace(s"HandleRPC($nodeKey): lookupAway($key, $moveAwayFrom, $numberOfNodes)")

      localRouting
        .lookupAway(key, moveAwayFrom, numberOfNodes)
        .toIO
    }
}
