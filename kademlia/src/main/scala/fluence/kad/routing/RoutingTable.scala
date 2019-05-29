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

package fluence.kad.routing

import cats.effect.{Async, Clock, Concurrent, LiftIO, Timer}
import cats.{Parallel, Traverse}
import cats.instances.list._
import cats.syntax.flatMap._
import cats.syntax.applicative._
import cats.syntax.functor._
import fluence.effects.kvstore.KVStore
import fluence.kad.protocol.{ContactAccess, Key, Node}
import fluence.kad.state.RoutingState
import fluence.log.Log

import scala.concurrent.duration.FiniteDuration
import scala.language.higherKinds

/**
 * Wraps all the routing state and logic
 *
 * @param local Local routing, used in iterative routing
 * @param iterative Iterative routing
 * @param state Internal state, used for routing
 * @tparam F Effect
 * @tparam C Contact
 */
case class RoutingTable[F[_], C](
  local: LocalRouting[F, C],
  iterative: IterativeRouting[F, C],
  private[kad] val state: RoutingState[F, C]
) {

  def nodeKey: Key = local.nodeKey

}

object RoutingTable {

  /**
   * Wraps the logic of applying an extension while building the [[RoutingTable]], see [[apply]]
   *
   * @param modifyState Modify state, prior to building on it
   * @param modifyLocal Modify local routing logic, prior to building on it
   * @param modifyIterative Modify iterative routing logic, prior to building on it
   * @tparam F Effect
   * @tparam C Contact
   */
  sealed class Extension[F[_], C](
    private[routing] val modifyState: RoutingState[F, C] ⇒ F[RoutingState[F, C]],
    private[routing] val modifyLocal: LocalRouting[F, C] ⇒ F[LocalRouting[F, C]],
    private[routing] val modifyIterative: IterativeRouting[F, C] ⇒ F[IterativeRouting[F, C]]
  )

  /**
   * Extension: see [[RoutingState.bootstrapWithStore]]
   *
   * @param store Store to bootstrap from, and to save nodes to
   */
  def bootstrapWithStore[F[_]: Concurrent: Clock: Log, C: ContactAccess](
    store: KVStore[F, Key, Node[C]]
  ): Extension[F, C] =
    new Extension[F, C](
      state ⇒ RoutingState.bootstrapWithStore(state, store),
      _.pure[F],
      _.pure[F]
    )

  /**
   * Extension: see [[RefreshingIterativeRouting]]
   *
   * @param refreshTimeout Frequency for each bucket's refresh, should be around an hour or more
   * @param refreshNeighbors How many neighbors to lookup on refresh
   * @param parallelism Refreshing parallelism, should be taken from KademliaConf
   */
  def refreshing[F[_]: Concurrent: Timer: Log, C](
    refreshTimeout: FiniteDuration,
    refreshNeighbors: Int,
    parallelism: Int
  ): Extension[F, C] =
    new Extension[F, C](
      _.pure[F],
      _.pure[F],
      iterative ⇒ RefreshingIterativeRouting(iterative, refreshTimeout, refreshNeighbors, parallelism)
    )

  /**
   * Build an in-memory Kademlia state, apply extensions on it
   *
   * @param nodeKey Current node's key
   * @param siblingsSize Number of siblings to store in [[fluence.kad.state.Siblings]] state
   * @param maxBucketSize Number of nodes to store in each [[fluence.kad.state.Bucket]] state
   * @param extensions Extensions to apply
   * @return Ready-to-use RoutingTable, expected to be a singleton
   */
  def apply[F[_]: Async: Clock: LiftIO, P[_], C: ContactAccess](
    nodeKey: Key,
    siblingsSize: Int,
    maxBucketSize: Int,
    extensions: Extension[F, C]*
  )(implicit P: Parallel[F, P]): F[RoutingTable[F, C]] =
    for {
      // Build a plain in-memory routing state
      st ← RoutingState.inMemory[F, P, C](nodeKey, siblingsSize, maxBucketSize)

      // Apply extensions to the state, use extended version then
      exts = extensions.toList
      state ← Traverse[List].foldLeftM(exts, st) {
        case (s, ext) ⇒ ext.modifyState(s)
      }

      // Extend local routing, using extended state
      loc = LocalRouting(state.nodeKey, state.siblings, state.bucket)
      local ← Traverse[List].foldLeftM(exts, loc) {
        case (l, ext) ⇒ ext.modifyLocal(l)
      }

      // Extend iterative routing, using extended local routing and state
      it = IterativeRouting(local, state)
      iterative ← Traverse[List].foldLeftM(exts, it) {
        case (i, ext) ⇒ ext.modifyIterative(i)
      }

      // Yield routing, aggregating all the extensions inside
    } yield RoutingTable(local, iterative, state)

}
