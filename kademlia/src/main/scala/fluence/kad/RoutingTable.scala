package fluence.kad

import cats.syntax.applicative._
import cats.syntax.applicativeError._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.syntax.monoid._
import cats.syntax.order._
import cats.syntax.show._
import cats.{ MonadError, Traverse }
import org.slf4j.LoggerFactory

import scala.collection.immutable.SortedSet
import scala.concurrent.duration.Duration
import scala.language.higherKinds

/**
 * RoutingTable describes how to route various requests over Kademlia network.
 * State is stored within [[Siblings]] and [[Bucket]], so there's no special case class.
 */
object RoutingTable {
  private val log = LoggerFactory.getLogger(getClass)

  implicit class ReadOps[C: Bucket.ReadOps: Siblings.ReadOps](nodeId: Key) {
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

      def combine(left: Stream[Node[C]], right: Stream[Node[C]], seen: Set[Key] = Set.empty): Stream[Node[C]] = (left, right) match {
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
  }

  implicit class WriteOps[F[_], C](nodeId: Key)(implicit
      BW: Bucket.WriteOps[F, C],
                                                SW: Siblings.WriteOps[F, C],
                                                ME: MonadError[F, Throwable]) {
    /**
     * Locates the bucket responsible for given contact, and updates it using given ping function
     *
     * @param node        Contact to update
     * @param rpc         Function that pings the contact to check if it's alive
     * @param pingTimeout Duration when no ping requests are made by the bucket, to avoid overflows
     * @return True if the node is saved into routing table
     */
    def update(node: Node[C], rpc: C ⇒ KademliaRPC[F, C], pingTimeout: Duration): F[Boolean] =
      if (nodeId === node.key) false.pure[F]
      else {
        //log.debug("Update node: " + node.key)
        for {
          // Update bucket, performing ping if necessary
          savedToBuckets ← BW.update((node.key |+| nodeId).zerosPrefixLen, node, rpc, pingTimeout)

          // Update siblings
          savedToSiblings ← SW.add(node)

        } yield savedToBuckets || savedToSiblings
      }

    // As we see nodes, update routing table
    // TODO: we can group nodes by bucketId, and run updates in parallel with Traverse
    private def updateList(
      pending:     List[Node[C]],
      rpc:         C ⇒ KademliaRPC[F, C],
      pingTimeout: Duration,
      checked:     List[Node[C]]         = Nil
    ): F[List[Node[C]]] =
      pending match {
        case a :: tail ⇒
          update(a, rpc, pingTimeout).flatMap { b ⇒
            updateList(tail, rpc, pingTimeout, a :: checked)
          }

        case Nil ⇒
          checked.pure[F]
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
     * @param pingTimeout Duration to prevent too frequent ping requests from buckets
     * @return
     */
    def lookupIterative(
      key:       Key,
      neighbors: Int,

      parallelism: Int,

      rpc: C ⇒ KademliaRPC[F, C],

      pingTimeout: Duration

    ): F[Seq[Node[C]]] = {
      // Import for Traverse
      import cats.instances.list._

      implicit val ordering: Ordering[Node[C]] = Node.relativeOrdering(key)

      case class AdvanceData(shortlist: SortedSet[Node[C]], probed: Set[Key], hasNext: Boolean)

      // Query $parallelism more nodes, looking for better results
      def advance(shortlist: SortedSet[Node[C]], probed: Set[Key]): F[AdvanceData] = {
        // Take $parallelism unvisited nodes to perform lookups on
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
          val remote0X = Traverse[List].sequence(handle.map { c ⇒
            rpc(c.contact).lookup(key, neighbors)
          }).map[List[Node[C]]](
            _.flatten
              .filterNot(c ⇒ updatedProbed(c.key)) // Filter away already seen nodes
          )

          remote0X
            .flatMap(updateList(_, rpc, pingTimeout)) // Update routing table
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
          val d #:: tail = data
          advance(d, probed).flatMap { updatedData ⇒
            if (!updatedData.hasNext) {
              iterate((collected ++ updatedData.shortlist).take(neighbors), updatedData.probed, tail)
            } else iterate(collected, updatedData.probed, tail append Stream(updatedData.shortlist))
          }
        }

      val shortlistEmpty = SortedSet.empty[Node[C]]

      // Perform local lookup
      val closestSeq0 = nodeId.lookup(key)
      val closest = closestSeq0.take(parallelism)

      // We perform lookup on $parallelism disjoint paths
      // To ensure paths are disjoint, we keep the sole set of visited contacts
      // To synchronize the set, we iterate over $parallelism distinct shortlists
      iterate(shortlistEmpty ++ closest, Set.empty, closest.map(shortlistEmpty + _))
    }.map(_.take(neighbors))

    /**
     * Joins network with known peers
     *
     * @param peers         List of known peer contacts (assuming that Kademlia ID is unknown)
     * @param rpc           RPC for remote nodes call
     * @param pingTimeout   Duration to avoid too frequent ping requests, used in [[Bucket.update()]]
     * @param numberOfNodes How many nodes to lookupIterative for each peer
     * @return F[Unit], possibly a failure if were not able to join any node
     */
    def join(peers: Seq[C], rpc: C ⇒ KademliaRPC[F, C], pingTimeout: Duration, numberOfNodes: Int): F[Unit] = {
      import cats.instances.list._

      Traverse[List].traverse(peers.toList) { peer: C ⇒
        // For each peer
        // Try to ping the peer; if no pings are performed, join is failed
        rpc(peer).ping().attempt.flatMap[Option[(Node[C], List[Node[C]])]] {
          case Right(peerNode) if peerNode.key =!= nodeId ⇒ // Ping successful, lookup node's neighbors
            log.info("PeerPing successful to " + peerNode.key)

            rpc(peer).lookupIterative(nodeId, numberOfNodes).attempt.map {
              case Right(neighbors) if neighbors.isEmpty ⇒
                log.info("Neighbors list is empty for peer " + peerNode.key)
                Some(peerNode -> Nil)

              case Right(neighbors) ⇒
                Some(peerNode -> neighbors.toList)

              case Left(e) ⇒
                log.warn(s"Can't perform lookup for $peer during join", e)
                Some(peerNode -> Nil)
            }

          case Right(_) ⇒
            log.debug("Can't initialize from myself")
            Option.empty[(Node[C], List[Node[C]])].pure[F]

          case Left(e) ⇒
            log.warn(s"Can't perform ping for $peer during join", e)
            Option.empty[(Node[C], List[Node[C]])].pure[F]
        }

      }.map(_.flatten)
        .flatMap { peerNeighbors ⇒

          val ps = peerNeighbors.map(_._1)
          val peerSet = ps.map(_.key).toSet

          val ns = peerNeighbors.flatMap(_._2).groupBy(_.key).mapValues(_.head).values.filterNot(n ⇒ peerSet(n.key)).toList

          Traverse[List].traverse(
            ns
          )(p ⇒ rpc(p.contact).ping().attempt).map(_.collect {
            case Right(n) ⇒ n
          }).map(_ ::: ps)

        }.flatMap { ns ⇒
          // Save discovered nodes to the routing table
          log.info("Discovered neighbors: " + ns.map(_.key))
          updateList(ns, rpc, pingTimeout)
        }.map(_.nonEmpty).flatMap {
          case true ⇒ // At least joined to a single node
            log.info("Joined! " + Console.GREEN + nodeId + Console.RESET)
            ().pure[F]
          case false ⇒ // Can't join to any node
            log.warn("Can't join!")
            ME.raiseError[Unit](new RuntimeException("Can't join any node among known peers"))
        }
    }
  }

}