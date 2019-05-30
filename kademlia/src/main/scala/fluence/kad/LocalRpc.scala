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
import cats.data.EitherT
import cats.syntax.apply._
import fluence.kad.routing.LocalRouting
import fluence.kad.protocol.{KademliaRpc, Key, Node}
import fluence.log.Log

import scala.language.higherKinds

/**
 * Handler for incoming Kademlia RPC requests
 *
 * @param loopbackContact This node's contact
 * @param localRouting Local routing table
 * @tparam F Effect, to be converted to IO
 * @tparam C Type for contact data
 */
private[kad] class LocalRpc[F[_]: Monad, C](loopbackContact: F[Node[C]], localRouting: LocalRouting[F, C])
    extends KademliaRpc[F, C] {
  import localRouting.nodeKey

  /**
   * Ping the contact, get its actual Node status, or fail.
   */
  override def ping()(implicit log: Log[F]): EitherT[F, KadRpcError, Node[C]] =
    EitherT.right(
      Log[F].trace(s"HandleRPC($nodeKey): ping")
        *> loopbackContact
    )

  /**
   * Perform a local lookup for a key, return K closest known nodes.
   *
   * @param key Key to lookup
   */
  override def lookup(key: Key, numberOfNodes: Int)(implicit log: Log[F]): EitherT[F, KadRpcError, Seq[Node[C]]] =
    EitherT.right(
      Log[F].trace(s"HandleRPC($nodeKey): lookup($key, $numberOfNodes)")
        *> localRouting.lookup(key, numberOfNodes)
    )

  /**
   * Perform a local lookup for a key, return K closest known nodes, going away from the second key.
   *
   * @param key Key to lookup
   */
  override def lookupAway(key: Key, moveAwayFrom: Key, numberOfNodes: Int)(
    implicit log: Log[F]
  ): EitherT[F, KadRpcError, Seq[Node[C]]] =
    EitherT.right(
      Log[F].trace(s"HandleRPC($nodeKey): lookupAway($key, $moveAwayFrom, $numberOfNodes)")
        *> localRouting.lookupAway(key, moveAwayFrom, numberOfNodes)
    )
}
