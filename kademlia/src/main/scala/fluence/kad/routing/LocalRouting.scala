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

import cats.effect.{Clock, Concurrent, IO, LiftIO}
import cats.instances.list._
import cats.syntax.applicative._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.syntax.apply._
import cats.syntax.semigroupk._
import cats.syntax.monoid._
import cats.syntax.order._
import cats.{Monad, Parallel, Traverse}
import fluence.effects.kvstore.KVStore
import fluence.kad.protocol.{KademliaRpc, Key, Node}
import fluence.kad.state.{Bucket, BucketsState, ModResult, Siblings, SiblingsState}

import scala.annotation.tailrec
import scala.collection.immutable.Queue
import scala.collection.mutable.ListBuffer
import scala.concurrent.duration.Duration
import scala.language.higherKinds

/**
 * LocalRouting describes how to route various requests over Kademlia network.
 * State is stored within [[Siblings]] and [[Bucket]], so there's no special case class.
 */
trait LocalRouting[F[_], C] {
  val nodeId: Key

  /**
   * Tries to route a key to a Contact, if it's known locally
   *
   * @param key Key to lookup
   */
  def find(key: Key): F[Option[Node[C]]]

  /**
   * Performs local lookup for the key, returning a stream of closest known nodes to it
   *
   * @param key Key to lookup
   * @return
   */
  def lookup(key: Key, numOfNodes: Int, predicate: Node[C] ⇒ Boolean = _ ⇒ true): F[Seq[Node[C]]]

  /**
   * Perform a lookup in local RoutingTable for a key,
   * return `numberOfNodes` closest known nodes, going away from the second key
   *
   * @param key Key to lookup
   * @param moveAwayFrom Key to move away
   */
  def lookupAway(key: Key, moveAwayFrom: Key, numOfNodes: Int): F[Seq[Node[C]]]

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

object LocalRouting {

  def apply[F[_]: Monad, P[_], C](nodeId: Key, siblings: SiblingsState[F, C], buckets: BucketsState[F, C])(
    implicit P: Parallel[F, P]
  ): LocalRouting[F, C] = new Impl(nodeId, siblings, buckets)

  private class Impl[F[_]: Monad, P[_], C](val nodeId: Key, siblings: SiblingsState[F, C], buckets: BucketsState[F, C])(
    implicit P: Parallel[F, P]
  ) extends LocalRouting[F, C] with slogging.LazyLogging {

    /**
     * Tries to route a key to a Contact, if it's known locally
     *
     * @param key Key to lookup
     */
    override def find(key: Key): F[Option[Node[C]]] =
      P sequential P.apply.map2( // TODO: it's enough to find one non-empty reply; is there any way to explain it?
        P parallel siblings.read.map(_.find(key)),
        P parallel buckets.read(nodeId |+| key).map(_.find(key))
      )(_ orElse _)

    /**
     * Performs local lookup for the key, returning a stream of closest known nodes to it
     *
     * @param key Key to lookup
     * @return
     */
    override def lookup(key: Key, numOfNodes: Int, predicate: Node[C] ⇒ Boolean = _ ⇒ true): F[Seq[Node[C]]] = {

      implicit val ordering: Ordering[Node[C]] = Node.relativeOrdering(key)

      // Build stream of neighbors, taken from buckets
      val bucketsStream: Stream[F[Stream[Node[C]]]] = {
        // Base index: nodes as far from this one as the target key is
        val idx = (nodeId |+| key).zerosPrefixLen

        // Diverging stream of indices, going left (farther from current node) then right (closer), like 5 4 6 3 7 ...
        Stream(idx)
          .filter(_ < Key.BitLength) // In case current node is given, this will remove IndexOutOfBoundsException
          .append(
            Stream.from(1).takeWhile(i ⇒ idx + i < Key.BitLength || idx - i >= 0).flatMap { i ⇒
              (if (idx - i >= 0) Stream(idx - i) else Stream.empty) append
                (if (idx + i < Key.BitLength) Stream(idx + i) else Stream.empty)
            }
          )
          .map(
            idx ⇒
              // Take contacts from the bucket, and sort them
              buckets.read(idx).map(_.stream)
          )
      }

      // Fetch the minimal necessary amount of nodes close to target from buckets
      def fetchEnoughFromBuckets(
        more: Stream[F[Stream[Node[C]]]],
        collected: Queue[Node[C]] = Queue.empty
      ): F[Queue[Node[C]]] =
        more match {
          case _ if collected.length >= numOfNodes ⇒ // TODO: this is not optimal
            collected.pure[F]
          case next #:: evenMore ⇒
            next.map(_.filter(predicate)).flatMap { fetched ⇒
              fetchEnoughFromBuckets(evenMore, collected enqueue fetched)
            }
          case Stream.Empty ⇒
            collected.pure[F]
        }

      def combine(left: Stream[Node[C]], right: Stream[Node[C]], seen: Set[Key] = Set.empty): Stream[Node[C]] =
        (left, right) match {
          case (hl #:: tl, _) if seen(hl.key) ⇒ combine(tl, right, seen)
          case (_, hr #:: tr) if seen(hr.key) ⇒ combine(left, tr, seen)
          case (hl #:: tl, hr #:: _) if ordering.lt(hl, hr) ⇒ hl #:: combine(tl, right, seen + hl.key)
          case (hl #:: _, hr #:: tr) if ordering.gt(hl, hr) ⇒ hr #:: combine(left, tr, seen + hr.key)
          case (hl #:: tl, hr #:: tr) if ordering.equiv(hl, hr) ⇒ hr #:: combine(tl, tr, seen + hr.key)
          case (Stream.Empty, _) ⇒ right
          case (_, Stream.Empty) ⇒ left
        }

      P sequential P.apply.map2(
        // Stream of neighbors, taken from siblings
        P parallel siblings.read.map(_.nodes.filter(predicate).toStream.sorted),
        // Stream of buckets, sorted by closeness
        P parallel fetchEnoughFromBuckets(bucketsStream).map(_.toStream.sorted)
      )(
        // Combine stream, taking closer nodes first
        combine(_, _).take(numOfNodes)
      )
    }

    /**
     * Perform a lookup in local RoutingTable for a key,
     * return `numberOfNodes` closest known nodes, going away from the second key
     *
     * @param key          Key to lookup
     * @param moveAwayFrom Key to move away
     */
    override def lookupAway(key: Key, moveAwayFrom: Key, numOfNodes: Int): F[Seq[Node[C]]] =
      lookup(key, numOfNodes, n ⇒ (n.key |+| key) < (n.key |+| moveAwayFrom))

    /**
     * Removes a node from routing table by its key, returns optional removed node
     *
     * @param key Key
     * @return Optional node, if it was removed
     */
    override def remove(key: Key): F[ModResult[C]] =
      P sequential P.apply.map2(
        P parallel siblings.remove(key),
        P parallel buckets.remove((key |+| nodeId).zerosPrefixLen, key)
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
                  P parallel buckets.update((node.key |+| nodeId).zerosPrefixLen, node, rpc, pingExpiresIn),
                  // Update siblings
                  P parallel siblings.add(node)
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
        .traverse(res.removed.toList)(find)
        .map(_.foldLeft(res) {
          case (mr, Some(n)) ⇒ mr.keep(n.key)
          case (mr, _) ⇒ mr
        })
  }

  /**
   * Load previously stored data from the store, bootstrap the LocalRouting with it; reflect LocalRouting state changes in the given store
   *
   * @param localRouting Local routing to delegate operations to
   * @param store Store for node contacts
   * @param rpc           Function to perform request to remote contact
   * @param pingExpiresIn Duration when no ping requests are made by the bucket, to avoid overflows
   * @param checkNode     Test node correctness, e.g. signatures are correct, ip is public, etc.
   * @tparam F Effect; storing is performed in the background with no blocking
   * @tparam C Contact
   * @return Bootstrapped LocalRouting that stores state changes in the store
   */
  def bootstrapWithStore[F[_]: Concurrent: Clock, C](
    localRouting: LocalRouting[F, C],
    store: KVStore[F, Key, Node[C]],
    rpc: C ⇒ KademliaRpc[C],
    pingExpiresIn: Duration,
    checkNode: Node[C] ⇒ IO[Boolean]
  ): F[LocalRouting[F, C]] =
    store.stream.chunks
      .evalTap(ch ⇒ localRouting.updateList(ch.map(_._2).toList, rpc, pingExpiresIn, checkNode).void)
      .compile
      .drain as withStore(localRouting, store)

  /**
   * Reflect LocalRouting's state modifications in KVStore
   *
   * @param localRouting Local routing to delegate operations to
   * @param store Store for node contacts
   * @tparam F Effect; storing is performed in the background with no blocking
   * @tparam C Contact
   * @return LocalRouting that stores state changes in the store
   */
  def withStore[F[_]: Concurrent, C](
    localRouting: LocalRouting[F, C],
    store: KVStore[F, Key, Node[C]]
  ): LocalRouting[F, C] =
    new LocalRouting[F, C] {
      // Reflect the state modification results in the store
      private def modResult(res: ModResult[C]): F[Unit] =
        Concurrent[F]
          .start(
            Traverse[List].traverse(res.updated.toList) {
              case (k, v) ⇒ store.put(k, v).value // TODO: at least log errors?
            } *> Traverse[List].traverse(res.removed.toList)(store.remove(_).value)
          )
          .void

      override val nodeId: Key =
        localRouting.nodeId

      override def find(key: Key): F[Option[Node[C]]] =
        localRouting.find(key)

      override def lookup(key: Key, numOfNodes: Int, predicate: Node[C] ⇒ Boolean): F[Seq[Node[C]]] =
        localRouting.lookup(key, numOfNodes, predicate)

      override def lookupAway(key: Key, moveAwayFrom: Key, numOfNodes: Int): F[Seq[Node[C]]] =
        localRouting.lookupAway(key, moveAwayFrom, numOfNodes)

      override def remove(key: Key): F[ModResult[C]] =
        localRouting.remove(key).flatTap(modResult)

      override def update(
        node: Node[C],
        rpc: C ⇒ KademliaRpc[C],
        pingExpiresIn: Duration,
        checkNode: Node[C] ⇒ IO[Boolean]
      )(implicit clock: Clock[F], liftIO: LiftIO[F]): F[ModResult[C]] =
        localRouting.update(node, rpc, pingExpiresIn, checkNode).flatTap(modResult)

      override def updateList(
        nodes: List[Node[C]],
        rpc: C ⇒ KademliaRpc[C],
        pingExpiresIn: Duration,
        checkNode: Node[C] ⇒ IO[Boolean]
      )(implicit clock: Clock[F], liftIO: LiftIO[F]): F[ModResult[C]] =
        localRouting.updateList(nodes, rpc, pingExpiresIn, checkNode).flatTap(modResult)
    }
}
