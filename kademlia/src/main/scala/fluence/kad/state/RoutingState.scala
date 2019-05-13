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

import cats.effect.{Async, Clock, Concurrent, LiftIO}
import cats.syntax.apply._
import cats.syntax.functor._
import cats.Parallel
import fluence.effects.kvstore.KVStore
import fluence.kad.protocol.{ContactAccess, Key, Node}

import scala.language.higherKinds

/**
 * Write model for routing state, hiding all the write access to both [[Siblings]] and [[Bucket]]
 *
 * @tparam F Effect
 * @tparam C Contact
 */
trait RoutingState[F[_], C] {

  def nodeKey: Key

  /**
   * Non-blocking read access for [[Siblings]] state
   */
  def siblings: F[Siblings[C]]

  /**
   * Non-blocking read access for a [[Bucket]] by the distance key
   *
   * @param distanceKey Usually (someOtherNodeKey |+| thisNodeKey)
   * @return Bucket
   */
  def bucket(distanceKey: Key): F[Bucket[C]]

  /**
   * Non-blocking read access for a [[Bucket]] by the given index; must be in range [0, [[Key.BitLength]])
   *
   * @param bucketId Bucket ID
   * @return Bucket
   */
  def bucket(bucketId: Int): F[Bucket[C]]

  /**
   * Removes a node from routing table by its key, returns optional removed node
   *
   * @param key Key
   * @return Optional node, if it was removed
   */
  def remove(key: Key): F[ModResult[C]]

  /**
   * Locates the bucket responsible for given contact, and updates it using given ping function
   *
   * @param node        Contact to update
   * @return True if the node is saved into routing table
   */
  def update(
    node: Node[C]
  )(implicit clock: Clock[F], liftIO: LiftIO[F], ca: ContactAccess[C]): F[ModResult[C]]

  /**
   * Update RoutingTable with a list of fresh nodes
   *
   * @param nodes List of new nodes
   * @return The same list of `nodes`
   */
  def updateList(
    nodes: List[Node[C]]
  )(implicit clock: Clock[F], liftIO: LiftIO[F], ca: ContactAccess[C]): F[ModResult[C]]
}

object RoutingState {

  /**
   * Initialize [[BucketsState]] and [[SiblingsState]] with [[ReadableMVar]], pass to [[RoutingStateImpl]]
   *
   * @param nodeKey This node's Kademlia key
   * @param siblingsSize How many siblings to store in the routing table
   * @param maxBucketSize The size of a bucket
   * @tparam C Contact
   */
  def inMemory[F[_]: Async, P[_], C](nodeKey: Key, siblingsSize: Int, maxBucketSize: Int)(
    implicit P: Parallel[F, P]
  ): F[RoutingState[F, C]] =
    (
      SiblingsState.withRef[F, C](nodeKey, siblingsSize),
      BucketsState.withMVar[F, C](maxBucketSize)
    ).mapN {
      case (ss, bs) ⇒ new RoutingStateImpl(nodeKey, ss, bs)
    }

  /**
   * Load previously stored data from the store, bootstrap the RoutingMutate with it; reflect RoutingMutate state changes in the given store
   *
   * @param routingMutate RoutingMutate to delegate operations to
   * @param store Store for node contacts
   * @tparam F Effect; storing is performed in the background with no blocking
   * @tparam C Contact
   * @return Bootstrapped RoutingState that stores state changes in the store
   */
  def bootstrapWithStore[F[_]: Concurrent: Clock, C: ContactAccess](
    routingMutate: RoutingState[F, C],
    store: KVStore[F, Key, Node[C]]
  ): F[RoutingState[F, C]] =
    store.stream.chunks
      .evalTap(ch ⇒ routingMutate.updateList(ch.map(_._2).toList).void)
      .compile
      .drain as withStore(routingMutate, store)

  /**
   * Reflect RoutingMutate's state modifications in KVStore
   *
   * @param routingState Routing mutate operations
   * @param store Store for node contacts
   * @tparam F Effect; storing is performed in the background with no blocking
   * @tparam C Contact
   * @return RoutingState that stores state changes in the store
   */
  def withStore[F[_]: Concurrent, C](
    routingState: RoutingState[F, C],
    store: KVStore[F, Key, Node[C]]
  ): RoutingState[F, C] =
    new StoredRoutingState[F, C](routingState, store)
}
