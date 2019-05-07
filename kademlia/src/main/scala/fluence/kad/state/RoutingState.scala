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

import cats.effect.{Async, Clock, Concurrent, IO, LiftIO}
import cats.instances.list._
import cats.syntax.applicative._
import cats.syntax.apply._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.syntax.monoid._
import cats.syntax.order._
import cats.syntax.semigroupk._
import cats.{Monad, Parallel, Traverse}
import fluence.effects.kvstore.KVStore
import fluence.kad.protocol.{KademliaRpc, Key, Node}
import fluence.kad.routing.LocalRouting

import scala.annotation.tailrec
import scala.collection.mutable.ListBuffer
import scala.concurrent.duration.Duration
import scala.language.higherKinds

trait RoutingState[F[_], C] {

  def siblings: F[Siblings[C]]

  def bucket(distanceKey: Key): F[Bucket[C]]

  def bucket(idx: Int): F[Bucket[C]]

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
   * @param rpc           Function to perform request to remote contact
   * @param pingExpiresIn Duration when no ping requests are made by the bucket, to avoid overflows
   * @param checkNode Test node correctness, e.g. signatures are correct, ip is public, etc.
   * @return True if the node is saved into routing table
   */
  def update(
    node: Node[C],
    rpc: C ⇒ KademliaRpc[C],
    pingExpiresIn: Duration,
    checkNode: Node[C] ⇒ IO[Boolean]
  )(implicit clock: Clock[F], liftIO: LiftIO[F]): F[ModResult[C]]

  /**
   * Update RoutingTable with a list of fresh nodes
   *
   * @param nodes List of new nodes
   * @param rpc   Function to perform request to remote contact
   * @param pingExpiresIn Duration when no ping requests are made by the bucket, to avoid overflows
   * @param checkNode Test node correctness, e.g. signatures are correct, ip is public, etc.
   * @return The same list of `nodes`
   */
  def updateList(
    nodes: List[Node[C]],
    rpc: C ⇒ KademliaRpc[C],
    pingExpiresIn: Duration,
    checkNode: Node[C] ⇒ IO[Boolean]
  )(implicit clock: Clock[F], liftIO: LiftIO[F]): F[ModResult[C]]
}

object RoutingState {

  /**
   * Initialize [[BucketsState]] and [[SiblingsState]] with [[ReadableMVar]], pass to [[Impl]]
   *
   * @param nodeId This node's Kademlia key
   * @param siblingsSize How many siblings to store in the routing table
   * @param maxBucketSize The size of a bucket
   * @tparam C Contact
   */
  def withMVar[F[_]: Async, P[_], C](nodeId: Key, siblingsSize: Int, maxBucketSize: Int)(
    implicit P: Parallel[F, P]
  ): F[RoutingState[F, C]] =
    (
      SiblingsState.withMVar[F, C](nodeId, siblingsSize),
      BucketsState.withMVar[F, C](maxBucketSize)
    ).mapN {
      case (ss, bs) ⇒ new Impl(nodeId, ss, bs)
    }

  private class Impl[F[_]: Monad, P[_], C](
    val nodeId: Key,
    siblingsState: SiblingsState[F, C],
    bucketsState: BucketsState[F, C]
  )(
    implicit P: Parallel[F, P]
  ) extends RoutingState[F, C] with slogging.LazyLogging {

    override val siblings: F[Siblings[C]] =
      siblingsState.read

    override def bucket(distanceKey: Key): F[Bucket[C]] =
      bucketsState.read(distanceKey)

    override def bucket(idx: Int): F[Bucket[C]] =
      bucketsState.read(idx)

    /**
     * Removes a node from routing table by its key, returns optional removed node
     *
     * @param key Key
     * @return Optional node, if it was removed
     */
    override def remove(key: Key): F[ModResult[C]] =
      P sequential P.apply.map2(
        P parallel siblingsState.remove(key),
        P parallel bucketsState.remove((key |+| nodeId).zerosPrefixLen, key)
      )(_ <+> _)

    /**
     * Locates the bucket responsible for given contact, and updates it using given ping function
     *
     * @param node          Contact to update
     * @param rpc           Function to perform request to remote contact
     * @param pingExpiresIn Duration when no ping requests are made by the bucket, to avoid overflows
     * @param checkNode     Test node correctness, e.g. signatures are correct, ip is public, etc.
     * @return ModResult
     */
    override def update(
      node: Node[C],
      rpc: C ⇒ KademliaRpc[C],
      pingExpiresIn: Duration,
      checkNode: Node[C] ⇒ IO[Boolean]
    )(implicit clock: Clock[F], liftIO: LiftIO[F]): F[ModResult[C]] =
      updateK(node, rpc, pingExpiresIn, checkNode, keepExisting = true)

    /**
     * Implementation for [[update]] with managed ModResult checks -- required for [[updateList]] optimization
     */
    private def updateK(
      node: Node[C],
      rpc: C ⇒ KademliaRpc[C],
      pingExpiresIn: Duration,
      checkNode: Node[C] ⇒ IO[Boolean],
      keepExisting: Boolean
    )(implicit clock: Clock[F], liftIO: LiftIO[F]): F[ModResult[C]] =
      if (nodeId === node.key)
        ModResult.noop[C].pure[F]
      else
        checkNode(node).attempt.to[F].flatMap {
          case Right(true) ⇒
            logger.trace("Update node: {}", node.key)

            P.sequential(
                P.apply.map2(
                  // Update bucket, performing ping if necessary
                  P parallel bucketsState.update((node.key |+| nodeId).zerosPrefixLen, node, rpc, pingExpiresIn),
                  // Update siblings
                  P parallel siblingsState.add(node)
                )(_ <+> _)
              )
              .flatMap(if (keepExisting) keepExistingNodes else _.pure[F])

          case Left(err) ⇒
            logger.trace(s"Node check failed with an exception for $node", err)
            ModResult.noop[C].pure[F]

          case _ ⇒
            ModResult.noop[C].pure[F]
        }

    /**
     * Update RoutingTable with a list of fresh nodes
     *
     * @param nodes         List of new nodes
     * @param rpc           Function to perform request to remote contact
     * @param pingExpiresIn Duration when no ping requests are made by the bucket, to avoid overflows
     * @param checkNode     Test node correctness, e.g. signatures are correct, ip is public, etc.
     * @return Result of state modification
     */
    override def updateList(
      nodes: List[Node[C]],
      rpc: C ⇒ KademliaRpc[C],
      pingExpiresIn: Duration,
      checkNode: Node[C] ⇒ IO[Boolean]
    )(implicit clock: Clock[F], liftIO: LiftIO[F]): F[ModResult[C]] = {
      // From iterable of groups, make list of list of items from different groups
      @tailrec
      def rearrange(groups: Iterable[List[Node[C]]], agg: List[List[Node[C]]] = Nil): List[List[Node[C]]] = {
        if (groups.isEmpty) agg
        else {
          val current = ListBuffer[Node[C]]()
          val next = ListBuffer[List[Node[C]]]()
          groups.foreach {
            case head :: Nil ⇒
              current.append(head)
            case head :: tail ⇒
              current.append(head)
              next.append(tail)
            case _ ⇒
          }
          rearrange(next.toList, current.toList :: agg)
        }
      }

      // Update portion, taking nodes one by one, and return all updated nodes
      def updatePortion(portion: List[Node[C]], agg: ModResult[C] = ModResult.noop): F[ModResult[C]] =
        portion match {
          case Nil ⇒ agg.pure[F]
          case node :: tail ⇒
            updateK(node, rpc, pingExpiresIn, checkNode, keepExisting = false).flatMap { res ⇒
              updatePortion(tail, res <+> agg)
            }
        }

      // Update each portion in parallel, and return all updated nodes
      def updateParPortions(portions: List[List[Node[C]]]): F[ModResult[C]] =
        Parallel.parTraverse(portions)(updatePortion(_)).map(_.foldLeft(ModResult.noop[C])(_ <+> _))

      updateParPortions(
        // Rearrange in portions with distinct bucket ids, so that it's possible to update it in parallel
        rearrange(
          // Group by bucketId, so that each group should never be updated in parallel
          nodes.groupBy(p ⇒ (p.key |+| nodeId).zerosPrefixLen).values
        )
      ).flatMap(keepExistingNodes)
    }

    /**
     * Keep existing nodes in case they were dropped either from buckets or siblings, but kept in another storage.
     *
     * @param res Mod result
     * @return ModResult
     */
    private def keepExistingNodes(res: ModResult[C]): F[ModResult[C]] =
      Traverse[List]
        .traverse(res.removed.toList)(
          k ⇒
            // TODO is it optimal?
            (
              siblingsState.read.map(_.find(k)),
              bucketsState.read(nodeId |+| k).map(_.find(k))
            ).mapN(_ orElse _)
        )
        .map(_.foldLeft(res) {
          case (mr, Some(n)) ⇒ mr.keep(n.key)
          case (mr, _) ⇒ mr
        })
  }

  /**
   * Load previously stored data from the store, bootstrap the RoutingMutate with it; reflect RoutingMutate state changes in the given store
   *
   * @param routingMutate RoutingMutate to delegate operations to
   * @param store Store for node contacts
   * @param rpc           Function to perform request to remote contact
   * @param pingExpiresIn Duration when no ping requests are made by the bucket, to avoid overflows
   * @param checkNode     Test node correctness, e.g. signatures are correct, ip is public, etc.
   * @tparam F Effect; storing is performed in the background with no blocking
   * @tparam C Contact
   * @return Bootstrapped RoutingState that stores state changes in the store
   */
  def bootstrapWithStore[F[_]: Concurrent: Clock, C](
    routingMutate: RoutingState[F, C],
    store: KVStore[F, Key, Node[C]],
    rpc: C ⇒ KademliaRpc[C],
    pingExpiresIn: Duration,
    checkNode: Node[C] ⇒ IO[Boolean]
  ): F[RoutingState[F, C]] =
    store.stream.chunks
      .evalTap(ch ⇒ routingMutate.updateList(ch.map(_._2).toList, rpc, pingExpiresIn, checkNode).void)
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
    new RoutingState[F, C] {

      override val siblings: F[Siblings[C]] =
        routingState.siblings

      override def bucket(distanceKey: Key): F[Bucket[C]] =
        routingState.bucket(distanceKey)

      override def bucket(idx: Int): F[Bucket[C]] =
        routingState.bucket(idx)

      // Reflect the state modification results in the store
      private def modResult(res: ModResult[C]): F[Unit] =
        Concurrent[F]
          .start(
            Traverse[List].traverse(res.updated.toList) {
              case (k, v) ⇒ store.put(k, v).value // TODO: at least log errors?
            } *> Traverse[List].traverse(res.removed.toList)(store.remove(_).value)
          )
          .void

      override def remove(key: Key): F[ModResult[C]] =
        routingState.remove(key).flatTap(modResult)

      override def update(
        node: Node[C],
        rpc: C ⇒ KademliaRpc[C],
        pingExpiresIn: Duration,
        checkNode: Node[C] ⇒ IO[Boolean]
      )(implicit clock: Clock[F], liftIO: LiftIO[F]): F[ModResult[C]] =
        routingState.update(node, rpc, pingExpiresIn, checkNode).flatTap(modResult)

      override def updateList(
        nodes: List[Node[C]],
        rpc: C ⇒ KademliaRpc[C],
        pingExpiresIn: Duration,
        checkNode: Node[C] ⇒ IO[Boolean]
      )(implicit clock: Clock[F], liftIO: LiftIO[F]): F[ModResult[C]] =
        routingState.updateList(nodes, rpc, pingExpiresIn, checkNode).flatTap(modResult)
    }
}
