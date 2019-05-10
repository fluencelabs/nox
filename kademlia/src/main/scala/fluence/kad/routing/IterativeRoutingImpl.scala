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

import cats.{Monad, Parallel}
import cats.data.EitherT
import cats.syntax.applicative._
import cats.syntax.eq._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.instances.list._
import cats.effect.{Clock, LiftIO}
import fluence.kad.{CantJoinAnyNode, JoinError}
import fluence.kad.protocol.{ContactAccess, Key, Node}
import fluence.kad.state.RoutingState

import scala.collection.immutable.SortedSet
import scala.language.higherKinds

/**
 * Iterative routing implementation
 *
 * @param localRouting Local routing table, used as a Read model for the routing state
 * @param routingState Routing state, used for state updates (as a Write model)
 * @tparam F Effect
 * @tparam C Contact
 */
private[routing] class IterativeRoutingImpl[F[_]: Monad: Clock: LiftIO, P[_], C: ContactAccess](
  localRouting: LocalRouting[F, C],
  routingState: RoutingState[F, C]
)(implicit P: Parallel[F, P])
    extends IterativeRouting[F, C] with slogging.LazyLogging {

  override def nodeId: Key = localRouting.nodeId

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
   * @return
   */
  override def lookupIterative(
    key: Key,
    neighbors: Int,
    parallelism: Int
  ): F[Seq[Node[C]]] = {
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
        val remote0X = Parallel
          .parTraverse(handle) { c ⇒
            ContactAccess[C]
              .rpc(c.contact)
              .lookup(key, neighbors)
              .attempt
              .map {
                case Left(err) ⇒
                  logger.warn(s"Cannot call lookupIterative on node $c", err)
                  Seq.empty
                case Right(sqnc) ⇒
                  sqnc
              }
              .to[F]
          }
          .map[List[Node[C]]](
            _.flatten
              .filterNot(c ⇒ updatedProbed(c.key)) // Filter away already seen nodes
          )

        remote0X
          .flatMap(routingState.updateList(_)) // Update routing table
          .map(_.updated.values.toList)
          .map { remotes ⇒
            val updatedShortlist = shortlist ++
              remotes.filter(
                c ⇒ (shortlist.size < neighbors || ordering.lt(c, shortlist.head)) && c.key =!= localRouting.nodeId
              )

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
    localRouting
      .lookup(key, parallelism)
      .map(_.toStream)
      .flatMap(
        closest ⇒ // TODO: why stream?

          // We perform lookup on `parallelism` disjoint paths
          // To ensure paths are disjoint, we keep the sole set of visited contacts
          // To synchronize the set, we iterate over `parallelism` distinct shortlists
          iterate(SortedSet(closest: _*), Set.empty, closest.map(SortedSet(_)))
      )
  }.map(_.take(neighbors))

  /**
   * Calls fn on some key's neighbourhood, described by ordering of `prefetchedNodes`,
   * until `numToCollect` successful replies are collected,
   * or `fn` is called `maxNumOfCalls` times,
   * or we can't find more nodes to try to call `fn` on.
   *
   * @param key            Key to call iterative nearby
   * @param fn             Function to call, should fail on error
   * @param numToCollect   How many successful replies should be collected
   * @param parallelism    Maximum expected parallelism
   * @param maxNumOfCalls  How many nodes may be queried with fn
   * @param isIdempotentFn For idempotent fn, more then `numToCollect` replies could be collected and returned;
   *                       should work faster due to better parallelism.
   *                       Note that due to network errors and timeouts you should never believe
   *                       that only the successfully replied nodes have actually changed its state.
   * @tparam A Return type
   * @return Pairs of unique nodes that has given reply, and replies.
   *         Size is <= `numToCollect` for non-idempotent `fn`,
   *         and could be up to (`numToCollect` + `parallelism` - 1) for idempotent fn.
   *         Size is lesser then `numToCollect` in case no more replies could be collected
   *         for one of the reasons described above.
   *         If size is >= `numToCollect`, call should be considered completely successful
   */
  override def callIterative[E, A](
    key: Key,
    fn: Node[C] ⇒ EitherT[F, E, A],
    numToCollect: Int,
    parallelism: Int,
    maxNumOfCalls: Int,
    isIdempotentFn: Boolean
  ): F[Vector[(Node[C], A)]] =
    lookupIterative(key, numToCollect max parallelism, parallelism).flatMap { prefetchedNodes ⇒
      // Lazy stream that takes nodes from the right
      def tailStream[T](from: SortedSet[T]): Stream[T] =
        from.toVector.reverseIterator.toStream

      // How many nodes to lookup, should be not too much to reduce network load,
      // and not too less to avoid network roundtrips
      // TODO: we should decide what value fits best; it's unknown if this formula is good enough
      val lookupSize = (parallelism max numToCollect) * parallelism

      // 1: take next nodes to try fn on.
      // Firstly take from seed, then expand seed with lookup on tail
      def moreNodes(
        loaded: SortedSet[Node[C]],
        lookedUp: Set[Key],
        loadMore: Int
      ): F[(SortedSet[Node[C]], Set[Key])] = {
        // If we can't expand the set, don't try
        if (lookedUp.size == loaded.size) (loaded, lookedUp).pure[F]
        else {
          // Take the most far nodes
          val toLookup = tailStream(loaded).filter(nc ⇒ !lookedUp(nc.key)).take(parallelism).toList

          // Make lookup requests for node's own neighborhood
          Parallel
            .parTraverse(toLookup) { n ⇒
              ContactAccess[C].rpc(n.contact).lookupAway(n.key, key, lookupSize).attempt.to[F]
            }
            .flatMap { lookupResult ⇒
              val ns = lookupResult.collect {
                case Right(v) ⇒ v
              }.flatten
              // Add new nodes, sort & filter dups with SortedSet
              val updatedLoaded = loaded ++ ns
              // Add keys used for neighborhood lookups to not lookup them again
              val updatedLookedUp = lookedUp ++ toLookup.map(_.key)
              // Thats the size of additions
              val loadedNum = updatedLoaded.size - loaded.size

              moreNodes(updatedLoaded, updatedLookedUp, loadMore - loadedNum)
              moreNodes(updatedLoaded, updatedLookedUp, loadMore - loadedNum)
            }
        }
      }

      // 2: on given nodes, call fn in parallel.
      // Return list of collected replies, and list of unsuccessful trials
      def callFn(nodes: List[Node[C]]): F[Seq[(Node[C], A)]] =
        Parallel
          .parTraverse(nodes)(n ⇒ fn(n).value.map(n -> _))
          .map(_.collect { case (n, Right(a)) ⇒ (n, a) })

      // 3: take nodes from 1, run 2, until one of conditions is met:
      // - numToCollect is collected
      // - maxRequests is made
      // - no more nodes to query are available
      def iterate(
        nodes: SortedSet[Node[C]],
        replies: Vector[(Node[C], A)],
        lookedUp: Set[Key],
        fnCalled: Set[Key],
        requestsRemaining: Int
      ): F[Vector[(Node[C], A)]] = {
        val needCollect = numToCollect - replies.size
        // If we've collected enough, stop
        if (needCollect <= 0) replies.pure[F]
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
              callFn(callOnNodes.toList).flatMap { newReplies ⇒
                val updatedReplies = replies ++ newReplies
                val updatedRequestsRemaining = requestsRemaining - updatedCallOnNodes.size
                val updatedFnCalled = fnCalled ++ updatedCallOnNodes.map(_.key)

                val escapeCondition =
                  updatedReplies.lengthCompare(numToCollect) >= 0 || // collected enough replies
                    updatedRequestsRemaining <= 0 || // Too many requests are made
                    (updatedFnCalled.size == updatedNodes.size && !hasMoreNodesToLookup) // No more nodes to call fn on

                if (escapeCondition)
                  updatedReplies.pure[F] // Stop iterations
                else
                  iterate(
                    updatedNodes,
                    updatedReplies,
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
        lookedUp = Set.empty,
        fnCalled = Set.empty,
        requestsRemaining = maxNumOfCalls
      )
    }

  /**
   * Joins network with known peers
   *
   * @param peers         List of known peer contacts (assuming that Kademlia ID is unknown)
   * @param numberOfNodes How many nodes to lookupIterative for each peer
   * @param parallelism   Parallelism factor to perform self-[[lookupIterative()]] in case of successful join
   * @return F[Unit], possibly a failure if were not able to join any node
   */
  override def join(
    peers: Seq[C],
    numberOfNodes: Int,
    parallelism: Int
  ): EitherT[F, JoinError, Unit] =
    EitherT(
      Parallel
        .parTraverse(peers.toList) { peer: C ⇒
          logger.trace("Going to ping Peer to join: " + peer)

          // For each peer
          // Try to ping the peer; if no pings are performed, join is failed
          ContactAccess[C].rpc(peer).ping().attempt.to[F].flatMap[Option[(Node[C], List[Node[C]])]] {
            case Right(peerNode) if peerNode.key =!= localRouting.nodeId ⇒ // Ping successful, lookup node's neighbors
              logger.info("PeerPing successful to " + peerNode.key)

              ContactAccess[C].rpc(peer).lookup(localRouting.nodeId, numberOfNodes).attempt.to[F].map {
                case Right(neighbors) if neighbors.isEmpty ⇒
                  logger.info("Neighbors list is empty for peer " + peerNode.key)
                  Some(peerNode -> Nil)

                case Right(neighbors) ⇒
                  Some(peerNode -> neighbors.toList)

                case Left(e) ⇒
                  logger.warn(s"Can't perform lookup for $peer during join", e)
                  Some(peerNode -> Nil)
              }

            case Right(_) ⇒
              logger.debug(s"Can't initialize from myself (${localRouting.nodeId})")
              Option.empty[(Node[C], List[Node[C]])].pure[F]

            case Left(e) ⇒
              logger.warn(s"Can't perform ping for $peer during join", e)
              Option.empty[(Node[C], List[Node[C]])].pure[F]
          }

        }
        .map(_.flatten)
        .flatMap { peerNeighbors ⇒
          val ps = peerNeighbors.map(_._1)
          val peerSet = ps.map(_.key).toSet

          val ns =
            peerNeighbors.flatMap(_._2).groupBy(_.key).mapValues(_.head).values.filterNot(n ⇒ peerSet(n.key)).toList

          Parallel
            .parTraverse(
              ns
            )(p ⇒ ContactAccess[C].rpc(p.contact).ping().attempt.to[F])
            .map(_.collect {
              case Right(n) ⇒ n
            })
            .map(_ ::: ps)

        }
        .flatMap { ns ⇒
          // Save discovered nodes to the routing table
          logger.info("Discovered neighbors: " + ns.map(_.key))
          routingState.updateList(ns)
        }
        .map(_.updated.nonEmpty)
        .flatMap[Either[JoinError, Unit]] {
          case true ⇒ // At least joined to a single node
            logger.info("Joined! " + Console.GREEN + localRouting.nodeId + Console.RESET)
            lookupIterative(localRouting.nodeId, numberOfNodes, numberOfNodes)
              .map(_ ⇒ Right(()))
          case false ⇒ // Can't join to any node
            logger.warn("Can't join!")
            Monad[F].pure(Left[JoinError, Unit](CantJoinAnyNode))
        }
    )
}
