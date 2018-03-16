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

import cats.data.{ EitherT, Ior, NonEmptyList }
import cats.syntax.applicative._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.syntax.monoid._
import cats.syntax.order._
import cats.instances.list._
import cats.{ Monad, Parallel }
import fluence.kad.protocol.{ KademliaRpc, Key, Node }

import scala.annotation.tailrec
import scala.collection.immutable.SortedSet
import scala.collection.mutable.ListBuffer
import scala.concurrent.duration.Duration
import scala.language.higherKinds

/**
 * RoutingTable describes how to route various requests over Kademlia network.
 * State is stored within [[Siblings]] and [[Bucket]], so there's no special case class.
 */
object RoutingTable {

  implicit class ReadOps[C : Bucket.ReadOps : Siblings.ReadOps](nodeId: Key) {
    private def SR = implicitly[Siblings.ReadOps[C]]

    private def BR = implicitly[Bucket.ReadOps[C]]

    /**
     * Tries to route a key to a Contact, if it's known locally
     *
     * @param key Key to lookup
     */
    def find(key: Key): Option[Node[C]] =
      SR.read.find(key).orElse(BR.read(nodeId |+| key).find(key))

    /**
     * Performs local lookup for the key, returning a stream of closest known nodes to it
     *
     * @param key Key to lookup
     * @return
     */
    def lookup(key: Key): Stream[Node[C]] = {

      implicit val ordering: Ordering[Node[C]] = Node.relativeOrdering(key)

      // Build stream of neighbors, taken from buckets
      val bucketsStream = {
        // Base index: nodes as far from this one as the target key is
        val idx = (nodeId |+| key).zerosPrefixLen

        // Diverging stream of indices, going left (far from current node) then right (closer), like 5 4 6 3 7 ...
        Stream(idx)
          .filter(_ < Key.BitLength) // In case current node is given, this will remove IndexOutOfBoundsException
          .append(Stream.from(1).takeWhile(i ⇒ idx + i < Key.BitLength || idx - i >= 0).flatMap { i ⇒
            (if (idx - i >= 0) Stream(idx - i) else Stream.empty) append
              (if (idx + i < Key.BitLength) Stream(idx + i) else Stream.empty)
          })
          .flatMap(idx ⇒
            // Take contacts from the bucket, and sort them
            BR.read(idx).stream
          )
      }

      // Stream of neighbors, taken from siblings
      val siblingsStream =
        SR.read.nodes.toStream.sorted

      def combine(left: Stream[Node[C]], right: Stream[Node[C]], seen: Set[Key] = Set.empty): Stream[Node[C]] =
        (left, right) match {
          case (hl #:: tl, _) if seen(hl.key)                   ⇒ combine(tl, right, seen)
          case (_, hr #:: tr) if seen(hr.key)                   ⇒ combine(left, tr, seen)
          case (hl #:: tl, hr #:: _) if ordering.lt(hl, hr)     ⇒ hl #:: combine(tl, right, seen + hl.key)
          case (hl #:: _, hr #:: tr) if ordering.gt(hl, hr)     ⇒ hr #:: combine(left, tr, seen + hr.key)
          case (hl #:: tl, hr #:: tr) if ordering.equiv(hl, hr) ⇒ hr #:: combine(tl, tr, seen + hr.key)
          case (Stream.Empty, _)                                ⇒ right
          case (_, Stream.Empty)                                ⇒ left
        }

      // Combine stream, taking closer nodes first
      combine(siblingsStream, bucketsStream)
    }

    /**
     * Perform a lookup in local RoutingTable for a key,
     * return `numberOfNodes` closest known nodes, going away from the second key
     *
     * @param key Key to lookup
     * @param moveAwayFrom Key to move away
     */
    def lookupAway(key: Key, moveAwayFrom: Key): Stream[Node[C]] =
      lookup(key).filter(n ⇒ (n.key |+| key) < (n.key |+| moveAwayFrom))
  }

  implicit class WriteOps[F[_], C](nodeId: Key)(implicit
      BW: Bucket.WriteOps[F, C],
      SW: Siblings.WriteOps[F, C],
      P: Parallel[F, F],
      F: Monad[F]) extends slogging.LazyLogging {
    /**
     * Locates the bucket responsible for given contact, and updates it using given ping function
     *
     * @param node        Contact to update
     * @param rpc           Function to perform request to remote contact
     * @param pingExpiresIn Duration when no ping requests are made by the bucket, to avoid overflows
     * @param checkNode Test node correctness, e.g. signatures are correct, ip is public, etc.
     * @return True if the node is saved into routing table
     */
    def update(node: Node[C], rpc: C ⇒ KademliaRpc[F, C], pingExpiresIn: Duration, checkNode: Node[C] ⇒ F[Boolean]): F[Boolean] =
      if (nodeId === node.key) false.pure[F]
      else {
        checkNode(node).flatMap {
          case true ⇒
            logger.trace("Update node: {}", node.key)
            for {
              // Update bucket, performing ping if necessary
              savedToBuckets ← BW.update((node.key |+| nodeId).zerosPrefixLen, node, rpc, pingExpiresIn)

              // Update siblings
              savedToSiblings ← SW.add(node)

            } yield savedToBuckets || savedToSiblings

          case false ⇒
            false.pure[F]
        }
      }

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
      rpc: C ⇒ KademliaRpc[F, C],
      pingExpiresIn: Duration,
      checkNode: Node[C] ⇒ F[Boolean]
    ): F[List[Node[C]]] = {
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

      // Update each portion in parallel
      def updateParPortions(portions: List[List[Node[C]]], agg: List[Node[C]] = Nil): F[List[Node[C]]] = portions match {
        case Nil         ⇒ agg.pure[F]
        case Nil :: tail ⇒ updateParPortions(tail, agg)
        case portion :: tail ⇒
          Parallel.parTraverse(portion)(update(_, rpc, pingExpiresIn, checkNode)).flatMap(_ ⇒
            updateParPortions(tail, portion ::: agg)
          )
      }

      updateParPortions(
        // Rearrange in portions with distinct bucket ids, so that it's possible to update it in parallel
        rearrange(
          // Group by bucketId, so that each group should never be updated in parallel
          nodes.groupBy(p ⇒ (p.key |+| nodeId).zerosPrefixLen).values
        )
      ).map(_ ⇒ nodes)
    }

    /**
     * The search begins by selecting alpha contacts from the non-empty k-bucket closest to the bucket appropriate
     * to the key being searched on. If there are fewer than alpha contacts in that bucket, contacts are selected
     * from other buckets. The contact closest to the target key, closestNode, is noted.
     *
     * The first alpha contacts selected are used to create a shortlist for the search.
     *
     * The node then sends parallel, asynchronous FIND_* RPCs to the alpha contacts in the shortlist.
     * Each contact, if it is live, should normally return k triples. If any of the alpha contacts fails to reply,
     * it is removed from the shortlist, at least temporarily.
     *
     * The node then fills the shortlist with contacts from the replies received. These are those closest to the target.
     * From the shortlist it selects another alpha contacts. The only condition for this selection is that they have not
     * already been contacted. Once again a FIND_* RPC is sent to each in parallel.
     *
     * Each such parallel search updates closestNode, the closest node seen so far.
     *
     * The sequence of parallel searches is continued until either no node in the sets returned is closer than the
     * closest node already seen or the initiating node has accumulated k probed and known to be active contacts.
     *
     * If a cycle doesn't find a closer node, if closestNode is unchanged, then the initiating node sends a FIND_* RPC
     * to each of the k closest nodes that it has not already queried.
     *
     * At the end of this process, the node will have accumulated a set of k active contacts or (if the RPC was FIND_VALUE)
     * may have found a data value. Either a set of triples or the value is returned to the caller.
     *
     * @param key         Key to find neighbors for
     * @param neighbors   A number of contacts to return
     * @param parallelism A number of requests performed in parallel
     * @param rpc         Function to perform request to remote contact
     * @param pingExpiresIn Duration to prevent too frequent ping requests from buckets
     * @param checkNode Test node correctness, e.g. signatures are correct, ip is public, etc.
     * @return
     */
    def lookupIterative(
      key: Key,
      neighbors: Int,

      parallelism: Int,

      rpc: C ⇒ KademliaRpc[F, C],

      pingExpiresIn: Duration,
      checkNode: Node[C] ⇒ F[Boolean]

    ): F[Seq[Node[C]]] = { // TODO: should we have KademliaRpc.Error there?
      // Import for Traverse
      import cats.instances.list._

      implicit val ordering: Ordering[Node[C]] = Node.relativeOrdering(key)

      case class AdvanceData(shortlist: SortedSet[Node[C]], probed: Set[Key], hasNext: Boolean)

      // Query `parallelism` more nodes, looking for better results
      def advance(shortlist: SortedSet[Node[C]], probed: Set[Key]): F[AdvanceData] = {
        // Take `parallelism` unvisited nodes to perform lookups on
        val handle = shortlist.filter(c ⇒ !probed(c.key)).take(parallelism).toList

        // If handle is empty, return
        if (handle.isEmpty || shortlist.isEmpty) {
          AdvanceData(shortlist, probed, hasNext = false).pure[F]
        } else {

          // The closest node -- we're trying to improve this result
          //val closest = shortlist.head

          // We're going to probe handled, and want to filter them out
          val updatedProbed = probed ++ handle.map(_.key)

          // Fetch remote lookups into F; filter previously seen nodes
          val remote0X = Parallel.parTraverse(handle) { c ⇒
            rpc(c.contact).lookup(key, neighbors).value.map(_.right.toOption)
          }.map[List[Node[C]]](
            _.collect{ case Some(remoteFound) ⇒ remoteFound }.flatten
              .filterNot(c ⇒ updatedProbed(c.key)) // Filter away already seen nodes
          )

          remote0X
            .flatMap(updateList(_, rpc, pingExpiresIn, checkNode)) // Update routing table
            .map {
              remotes ⇒
                val updatedShortlist = shortlist ++
                  remotes.filter(c ⇒ (shortlist.size < neighbors || ordering.lt(c, shortlist.head)) && c.key =!= nodeId)

                AdvanceData(updatedShortlist, updatedProbed, hasNext = true)
            }
        }
      }

      def iterate(collected: SortedSet[Node[C]], probed: Set[Key], data: Stream[SortedSet[Node[C]]]): F[Seq[Node[C]]] =
        if (data.isEmpty) collected.toSeq.pure[F]
        else {
          logger.debug("Iterate over: " + collected.map(_.contact))
          val d #:: tail = data
          advance(d, probed).flatMap { updatedData ⇒
            if (!updatedData.hasNext) {
              iterate((collected ++ updatedData.shortlist).take(neighbors), updatedData.probed, tail)
            } else iterate(collected, updatedData.probed, tail append Stream(updatedData.shortlist))
          }
        }

      // Perform local lookup
      val closestSeq0 = nodeId.lookup(key)
      val closest = closestSeq0.take(parallelism)

      // We perform lookup on `parallelism` disjoint paths
      // To ensure paths are disjoint, we keep the sole set of visited contacts
      // To synchronize the set, we iterate over `parallelism` distinct shortlists
      iterate(SortedSet(closest: _*), Set.empty, closest.map(SortedSet(_)))
    }.map(_.take(neighbors))

    /**
     * Calls fn on some key's neighbourhood, described by ordering of `prefetchedNodes`,
     * until `numToCollect` successful replies are collected,
     * or `fn` is called `maxNumOfCalls` times,
     * or we can't find more nodes to try to call `fn` on.
     *
     * @param key Key to call iterative nearby
     * @param fn Function to call, should fail on error
     * @param numToCollect How many successful replies should be collected
     * @param parallelism Maximum expected parallelism
     * @param maxNumOfCalls How many nodes may be queried with fn
     * @param isIdempotentFn For idempotent fn, more then `numToCollect` replies could be collected and returned;
     *                       should work faster due to better parallelism.
     *                       Note that due to network errors and timeouts you should never believe
     *                       that only the successfully replied nodes have actually changed its state.
     * @param rpc Used to perform lookups
     * @param pingExpiresIn Duration to prevent too frequent ping requests from buckets
     * @param checkNode Test node correctness, e.g. signatures are correct, ip is public, etc.
     * @tparam E Error type
     * @tparam A Return type
     * @return Pairs of unique nodes that has given reply, and replies.
     *         Size is <= `numToCollect` for non-idempotent `fn`,
     *         and could be up to (`numToCollect` + `parallelism` - 1) for idempotent fn.
     *         Size is lesser then `numToCollect` in case no more replies could be collected
     *         for one of the reasons described above.
     *         If size is >= `numToCollect`, call should be considered completely successful
     */
    def callIterative[E, A](
      key: Key,
      fn: Node[C] ⇒ EitherT[F, E, A],
      numToCollect: Int,
      parallelism: Int,
      maxNumOfCalls: Int,
      isIdempotentFn: Boolean,

      rpc: C ⇒ KademliaRpc[F, C],
      pingExpiresIn: Duration,
      checkNode: Node[C] ⇒ F[Boolean]
    ): EitherT[F, Seq[(Node[C], E)], Seq[(Node[C], A)]] =
      EitherT apply lookupIterative(key, numToCollect max parallelism, parallelism, rpc, pingExpiresIn, checkNode).flatMap {
        prefetchedNodes ⇒

          // Lazy stream that takes nodes from the right
          def tailStream[T](from: SortedSet[T]): Stream[T] =
            from.lastOption.fold(Stream.empty[T])(last ⇒ Stream.cons(last, tailStream(from.init)))

          // How many nodes to lookup, should be not too much to reduce network load,
          // and not too less to avoid network roundtrips
          // TODO: we should decide what value fits best; it's unknown if this formula is good enough
          val lookupSize = (parallelism max numToCollect) * parallelism

          // 1: take next nodes to try fn on.
          // Firstly take from seed, then expand seed with lookup on tail
          def moreNodes(loaded: SortedSet[Node[C]], lookedUp: Set[Key], loadMore: Int): F[(SortedSet[Node[C]], Set[Key])] = {
            // If we can't expand the set, don't try
            if (lookedUp.size == loaded.size) (loaded, lookedUp).pure[F]
            else {
              // Take the most far nodes
              val toLookup = tailStream(loaded).filter(nc ⇒ !lookedUp(nc.key)).take(parallelism).toList

              // Make lookup requests for node's own neighborhood
              Parallel.parTraverse(toLookup){ n ⇒ rpc(n.contact).lookupAway(n.key, key, lookupSize).value.map(_.right.toOption) }.flatMap {
                lookupResult ⇒
                  val ns = lookupResult.collect {
                    case Some(v) ⇒ v
                  }.flatten
                  // Add new nodes, sort & filter dups with SortedSet
                  val updatedLoaded = loaded ++ ns
                  // Add keys used for neighborhood lookups to not lookup them again
                  val updatedLookedUp = lookedUp ++ toLookup.map(_.key)
                  // Thats the size of additions
                  val loadedNum = updatedLoaded.size - loaded.size

                  moreNodes(updatedLoaded, updatedLookedUp, loadMore - loadedNum)
              }
            }
          }

          // 2: on given nodes, call fn in parallel.
          // Return list of collected replies, and list of unsuccessful trials
          def callFn(nodes: List[Node[C]]): F[List[(Node[C], Either[E, A])]] =
            Parallel.parTraverse(nodes)(n ⇒ fn(n).value.map(n -> _))

          // 3: take nodes from 1, run 2, until one of conditions is met:
          // - numToCollect is collected
          // - maxRequests is made
          // - no more nodes to query are available
          def iterate(
            nodes: SortedSet[Node[C]],
            replies: Seq[(Node[C], A)],
            errors: Seq[(Node[C], E)],
            lookedUp: Set[Key],
            fnCalled: Set[Key],
            requestsRemaining: Int): F[(Seq[(Node[C], E)], Seq[(Node[C], A)])] = {
            val needCollect = numToCollect - replies.size
            // If we've collected enough, stop
            if (needCollect <= 0) (errors, replies).pure[F]
            else {
              // For idempotent requests, we could make more calls then needed to increase chances to success
              val callsNeeded = if (isIdempotentFn) parallelism else needCollect min parallelism

              // Call on nodes
              val callOnNodes = nodes
                .filter(n ⇒ !fnCalled(n.key))
                .take(callsNeeded)

              (if (callOnNodes.size < callsNeeded) {
                // If there's not enough nodes to call fn on, try to get more
                moreNodes(nodes, lookedUp, needCollect - callOnNodes.size).map {
                  case (updatedNodes, updatedLookedUp) ⇒
                    (
                      updatedNodes,
                      updatedLookedUp,
                      updatedNodes.size - nodes.size >= needCollect - callOnNodes.size, // if there're new nodes, we have a reason to fetch more
                      updatedNodes
                      .filter(n ⇒ !fnCalled(n.key))
                      .take(callsNeeded)
                    )
                }
              } else {
                (nodes, lookedUp, true, callOnNodes).pure[F]
              }).flatMap {
                case (updatedNodes, updatedLookedUp, hasMoreNodesToLookup, updatedCallOnNodes) ⇒

                  callFn(callOnNodes.toList).flatMap {
                    newRepliesOrErrors ⇒
                      val newReplies = newRepliesOrErrors.collect {
                        case (n, Right(r)) ⇒ (n, r)
                      }

                      val newErrors = newRepliesOrErrors.collect {
                        case (n, Left(err)) ⇒ (n, err)
                      }

                      val updatedReplies = replies ++ newReplies
                      lazy val updatedErrors = errors ++ newErrors
                      val updatedRequestsRemaining = requestsRemaining - updatedCallOnNodes.size
                      val updatedFnCalled = fnCalled ++ updatedCallOnNodes.map(_.key)

                      val escapeCondition =
                        updatedReplies.lengthCompare(numToCollect) >= 0 || // collected enough replies
                          updatedRequestsRemaining <= 0 || // Too many requests are made
                          (updatedFnCalled.size == updatedNodes.size && !hasMoreNodesToLookup) // No more nodes to call fn on

                      if (escapeCondition)
                        (updatedErrors, updatedReplies).pure[F] // Stop iterations
                      else
                        iterate(
                          updatedNodes,
                          updatedReplies,
                          updatedErrors,
                          updatedLookedUp,
                          updatedFnCalled,
                          updatedRequestsRemaining
                        )
                  }
              }
            }
          }

          // Call with initial params
          iterate(
            nodes = SortedSet(prefetchedNodes: _*)(Node.relativeOrdering(key)),
            replies = Vector.empty,
            errors = Vector.empty,
            lookedUp = Set.empty,
            fnCalled = Set.empty,
            requestsRemaining = maxNumOfCalls
          ).map {
              case (errors, replies) ⇒
                Either.cond(replies.lengthCompare(numToCollect) >= 0, replies, errors)
            }
      }

    /**
     * Joins network with known peers
     *
     * @param peers         List of known peer contacts (assuming that Kademlia ID is unknown)
     * @param rpc           RPC for remote nodes call
     * @param pingExpiresIn   Duration to avoid too frequent ping requests, used in [[Bucket.update()]]
     * @param numberOfNodes How many nodes to lookupIterative for each peer
     * @param checkNode Test node correctness, e.g. signatures are correct, ip is public, etc.
     * @param parallelism Parallelism factor to perform self-[[lookupIterative()]] in case of successful join
     * @return Either a non-empty list of errors, or the discovered node's neighborhood (up to numberOfNodes)
     */
    def join[E](
      peers: Seq[C],
      rpc: C ⇒ KademliaRpc.Aux[F, C, E],
      pingExpiresIn: Duration,
      numberOfNodes: Int,
      checkNode: Node[C] ⇒ F[Boolean],
      parallelism: Int): EitherT[F, NonEmptyList[Either[E, JoinError]], Seq[Node[C]]] =
      {
        // For each peer contact,
        // get an error,
        // or peer Node[C] and its looked up nodeId neighborhood,
        // or both (in case lookup fails)
        def discoverPeer(peer: C): F[Ior[JoinError, (Node[C], List[Node[C]])]] = {
          logger.trace("Going to ping Peer to join: " + peer)

          // For each peer
          // Try to ping the peer; if no pings are performed, join is failed
          rpc(peer).ping().value.flatMap[Ior[JoinError, (Node[C], List[Node[C]])]] {
            case Right(peerNode) if peerNode.key =!= nodeId ⇒ // Ping successful, lookup node's neighbors
              logger.info("PeerPing successful to " + peerNode.key)

              // Perform lookup, map errors -- `.value` is just like `.attempt` call here
              rpc(peer).lookup(nodeId, numberOfNodes).value.map {
                case Right(neighbors) if neighbors.isEmpty ⇒
                  logger.info("Neighbors list is empty for peer " + peerNode.key)
                  Ior.right(peerNode -> Nil)

                case Right(neighbors) ⇒
                  Ior.right(peerNode -> neighbors.toList)

                case Left(e) ⇒
                  logger.warn(s"Can't perform lookup for $peer during join", e)
                  Ior.both(CantLookupContact(peer), peerNode -> Nil)
              }

            case Right(_) ⇒
              logger.debug("Can't initialize from myself")
              F.pure(Ior.left(CantJoinMyself))

            case Left(e) ⇒
              logger.debug(s"Can't perform ping for $peer during join", e)
              F.pure(Ior.left(CantPingContact(peer)))
          }
        }

        // Ping discovered neighbors, collect errors from previous step and this one
        def pingNeighbors(peerIorNeighbors: List[Ior[JoinError, (Node[C], List[Node[C]])]]): F[(List[Either[E, JoinError]], List[Node[C]])] = {
          val peerNeighbors = peerIorNeighbors.collect {
            case Ior.Right(r)   ⇒ r
            case Ior.Both(_, r) ⇒ r
          }

          val ps = peerNeighbors.map(_._1)
          val peerSet = ps.map(_.key).toSet

          val ns = peerNeighbors.flatMap(_._2).groupBy(_.key).mapValues(_.head).values.filterNot(n ⇒ peerSet(n.key)).toList

          Parallel.parTraverse(
            ns
          )(p ⇒ rpc(p.contact).ping().value)
            .map {
              listOfPingEither ⇒
                (
                  // All errors on the left (could be an empty list)
                  peerIorNeighbors.map(_.left).collect {
                    case Some(err) ⇒ Right[E, JoinError](err)
                  } ::: listOfPingEither.collect {
                    case Left(err) ⇒ Left[E, JoinError](err)
                  },
                  // All discovered neighbors on the right (could be an empty list)
                  listOfPingEither.collect {
                    case Right(n) ⇒ n
                  } ::: ps
                )
            }
        }

        // Update discovered neighbors in RoutingTable, return Ior with collected errors and/or discovered neighbors
        def updateNeighbors(errs: List[Either[E, JoinError]], discoveredNeighbors: List[Node[C]]): F[Ior[NonEmptyList[Either[E, JoinError]], List[Node[C]]]] = {
          // Save discovered nodes to the routing table
          val updatedNeighborsF: F[List[Node[C]]] =
            if (discoveredNeighbors.nonEmpty) {
              logger.info("Discovered neighbors: " + discoveredNeighbors.map(_.key))
              updateList(discoveredNeighbors, rpc, pingExpiresIn, checkNode)

            } else {
              F.pure(Nil)
            }

          // Manage error state
          updatedNeighborsF.map {
            case uns if uns.nonEmpty ⇒
              NonEmptyList.fromList(errs) match {
                case Some(errsNel) ⇒ Ior.both(errsNel, uns)
                case None          ⇒ Ior.right(uns)
              }

            case _ ⇒
              Ior.left(NonEmptyList(Right(CantJoinAnyContact), errs))
          }
        }

        def lookupOrFail(res: Ior[NonEmptyList[Either[E, JoinError]], Seq[Node[C]]]): F[Either[NonEmptyList[Either[E, JoinError]], Seq[Node[C]]]] =
          res match {
            case r if r.isRight ⇒ // At least joined to a single node
              logger.info("Joined! " + Console.GREEN + nodeId + Console.RESET)
              r.left.foreach { errs ⇒
                logger.warn("However, these errors occured during join: " + errs.toList.mkString(", "))
              }
              lookupIterative(nodeId, numberOfNodes, numberOfNodes, rpc, pingExpiresIn, checkNode)
                .map(Right[NonEmptyList[Either[E, JoinError]], Seq[Node[C]]])

            case r ⇒ // Can't join to any node
              logger.error("Can't join! " + r.left.toList.flatMap(_.toList).mkString(", "))
              F.pure(Left[NonEmptyList[Either[E, JoinError]], Seq[Node[C]]](r.left.get))
          }

        // Composition of all steps
        val joinF: F[Either[NonEmptyList[Either[E, JoinError]], Seq[Node[C]]]] =
          for {
            // For each peer contact, discover peer and lookup its neighbors
            peerIorNeighbors ← Parallel.parTraverse(peers.toList)(discoverPeer)

            // For each discovered neighbor, ping
            errsAndDiscoveredNeighbors ← pingNeighbors(peerIorNeighbors)
            (errs, discoveredNeighbors) = errsAndDiscoveredNeighbors

            // Update RoutingTable with discovered neighbors
            errsAndUpdatedNeighbors ← updateNeighbors(errs, discoveredNeighbors)

            // In case of any successful discovery, lookupIterative this node's neighborhood; or fail
            result ← lookupOrFail(errsAndUpdatedNeighbors)
          } yield result

        EitherT(joinF)
      }
  }

  sealed trait JoinError
  case object CantJoinMyself extends JoinError
  case class CantPingContact[C](contact: C) extends JoinError
  case class CantLookupContact[C](contact: C) extends JoinError
  case object CantJoinAnyContact extends JoinError

}
