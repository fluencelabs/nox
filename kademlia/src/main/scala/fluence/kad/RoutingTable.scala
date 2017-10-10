package fluence.kad

import cats.{Applicative, Id, Monad, MonadError, Show, Traverse}
import cats.data.StateT
import cats.syntax.monoid._
import cats.syntax.order._
import cats.syntax.applicative._
import cats.syntax.show._
import cats.syntax.functor._

import scala.collection.immutable.SortedSet
import scala.language.higherKinds

case class RoutingTable private(
                                 nodeId: Key,
                                 buckets: IndexedSeq[Bucket],
                                 siblings: SortedSet[Contact],
                                 maxSiblings: Int
                               )

object RoutingTable {

  implicit def show(implicit bs: Show[Bucket], ks: Show[Key]): Show[RoutingTable] =
    rt => s"RoutingTable(${ks.show(rt.nodeId)})" +
      rt.buckets.zipWithIndex.filter(_._1.nonEmpty).map {
        case (b, i) => s"$i(${Integer.toBinaryString(i)}):${bs.show(b)}"
      }.mkString(":\n", "\n", "") +
      rt.siblings.toSeq.map(_.key).map(ks.show).mkString(s"\nSiblings: ${rt.siblings.size}\n\t", "\n\t", "")

  def apply(nodeId: Key, maxBucketSize: Int, maxSiblings: Int) =
    new RoutingTable(
      nodeId,
      Vector.fill(Key.BitLength)(Bucket(maxBucketSize)),
      SortedSet.empty[Contact]((x, y) => (x.key |+| nodeId) compare (y.key |+| nodeId)),
      maxSiblings
    )

  /**
    * Just returns the table with given F effect
    *
    * @tparam F StateT effect
    * @return
    */
  def table[F[_] : Applicative]: StateT[F, RoutingTable, RoutingTable] =
    StateT.get

  /**
    * Locates the bucket responsible for given contact, and updates it using given ping function
    *
    * @param contact Contact to update
    * @param ping    Function that pings the contact to check if it's alive
    * @param ME      Monad error instance
    * @tparam F StateT effect
    * @return
    */
  def update[F[_]](contact: Contact, ping: Contact => F[Contact])(implicit ME: MonadError[F, Throwable]): StateT[F, RoutingTable, Unit] =
    table[F].flatMap { rt =>
      if (rt.nodeId === contact.key) StateT.pure(())
      else {
        val idx = (contact.key |+| rt.nodeId).zerosPrefixLen

        val bucket = rt.buckets(idx)

        for {
        // Update bucket, performing ping if necessary
          updatedBucketUnit <- StateT.lift(
            Bucket
              .update[F](contact, ping)
              .run(bucket)
          )

          // Save updated bucket to routing table
          _ <- StateT.set(rt.copy(
            buckets = rt.buckets.updated(idx, updatedBucketUnit._1),
            siblings = (rt.siblings + contact).take(rt.maxSiblings)
          ))
        } yield ()
      }
    }

  // As we see nodes, update routing table
  def updateList[F[_]](
                        pending: List[Contact],
                        ping: Contact => F[Contact],
                        checked: List[Contact] = Nil
                      )(implicit ME: MonadError[F, Throwable]): StateT[F, RoutingTable, List[Contact]] =
    pending match {
      case a :: tail =>
        update(a, ping).flatMap(_ =>
          updateList(tail, ping, a :: checked)
        )

      case Nil =>
        StateT.pure(checked)
    }

  /**
    * Tries to route a key to a Contact, if it's known locally
    *
    * @param key Key to lookup
    * @tparam F StateT effect
    * @return
    */
  def find[F[_] : Monad](key: Key): StateT[F, RoutingTable, Option[Contact]] =
    table[F].flatMap { rt =>
      if (rt.nodeId === key) StateT lift Option.empty[Contact].pure[F]
      else {
        // Lookup sibling, then buckets if nothing found
        findSibling(key).flatMapF {
          case r if r.isDefined => r.pure[F]
          case _ =>
            val idx = (key |+| rt.nodeId).zerosPrefixLen
            Bucket.find[F](key).run(rt.buckets(idx)).map(_._2)
        }
      }
    }

  /**
    * Tries to find a local sibling of this node
    *
    * @param key Key to lookup
    * @tparam F StateT effect
    * @return
    */
  def findSibling[F[_] : Monad](key: Key): StateT[F, RoutingTable, Option[Contact]] =
    table[F].map(rt =>
      rt.siblings.find(_.key === key)
    )

  /**
    * Performs local lookup for the key, returning a stream of closest known nodes to it
    *
    * @param key Key to lookup
    * @tparam F StateT effect
    * @return
    */
  def lookup[F[_] : Monad](key: Key): StateT[F, RoutingTable, Stream[Contact]] =
    table[F].flatMap { rt =>

      implicit val ordering: Ordering[Contact] = Contact.relativeOrdering(key)

      // Build stream of neighbors, taken from buckets
      val bucketsStream = {
        // Base index: nodes as far from this one as the target key is
        val idx = (rt.nodeId |+| key).zerosPrefixLen

        // Diverging stream of indices, going left (far from current node) then right (closer), like 5 4 6 3 7 ...
        Stream(idx)
          .filter(_ < Key.BitLength) // In case current node is given, this will remove IndexOutOfBoundsException
          .append(Stream.from(1).takeWhile(i => idx + i < Key.BitLength || idx - i >= 0).flatMap { i =>
          (if (idx - i >= 0) Stream(idx - i) else Stream.empty) append
            (if (idx + i < Key.BitLength) Stream(idx + i) else Stream.empty)
        })
          .flatMap(
            // Take contacts from the bucket, and sort them
            rt.buckets(_)
              .contacts
              .sorted
              .toStream
          )
      }

      // Stream of neighbors, taken from siblings
      val siblingsStream =
        rt.siblings.toStream.sorted

      def combine(left: Stream[Contact], right: Stream[Contact]): Stream[Contact] = (left, right) match {
        case (hl #:: tl, hr #:: _) if ordering.lt(hl, hr) => Stream(hl).append(combine(tl, right))
        case (hl #:: _, hr #:: tr) if ordering.gt(hl, hr) => Stream(hr).append(combine(left, tr))
        case (hl #:: tl, hr #:: tr) if ordering.equiv(hl, hr) => Stream(hr).append(combine(tl, tr))
        case (Stream.Empty, _) => right
        case (_, Stream.Empty) => left
      }

      // Combine stream
      val combined = combine(siblingsStream, bucketsStream)


      StateT pure combined
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
    * @param key          Key to find neighbors for
    * @param neighbors    A number of contacts to return
    * @param parallelism  A number of requests performed in parallel
    * @param ping         Function to perform pings, when updating routing table with newly seen nodes
    * @param lookupRemote Function to perform request to remote contact
    * @param ME           Monad Error for StateT effect
    * @tparam F StateT effect
    * @return
    */
  def lookupIterative[F[_]](
                             key: Key,
                             neighbors: Int,

                             parallelism: Int,

                             ping: Contact => F[Contact],
                             lookupRemote: (Contact, Key, Int) => F[Seq[Contact]]

                           )(implicit ME: MonadError[F, Throwable], sk: Show[Key]): StateT[F, RoutingTable, Seq[Contact]] = {
    // Import for Traverse
    import cats.instances.list._

    implicit val ordering: Ordering[Contact] = Contact.relativeOrdering(key)

    case class AdvanceData(shortlist: SortedSet[Contact], probed: Set[String], hasNext: Boolean)

    // Query $parallelism more nodes, looking for better results
    def advance(shortlist: SortedSet[Contact], probed: Set[String]): StateT[F, RoutingTable, AdvanceData] = {
      // Take $parallelism unvisited nodes to perform lookups on
      val handle = shortlist.filter(c => !probed(c.key.show)).take(parallelism).toList

      // If handle is empty, return
      if (handle.isEmpty || shortlist.isEmpty) {
        StateT.pure[F, RoutingTable, AdvanceData](AdvanceData(shortlist, probed, hasNext = false))
      } else {

        // The closest node -- we're trying to improve this result
        //val closest = shortlist.head

        // We're going to probe handled, and want to filter them out
        val updatedProbed = probed ++ handle.map(_.key.show)

        // Fetch remote lookups into F; filter previously seen nodes
        val remote0X = Traverse[List].sequence(handle.map { c =>
          lookupRemote(c, key, neighbors)
        }).map[List[Contact]](
          _.flatten
            .filterNot(c => updatedProbed(c.key.show)) // Filter away already seen nodes
        )

        StateT.lift[F, RoutingTable, List[Contact]](remote0X)
          .flatMap(updateList(_, ping)) // Update routing table
          .map {
          remotes =>
            val updatedShortlist = shortlist ++
              remotes.filter(c => shortlist.size < neighbors || (c.key |+| key) < (shortlist.head.key |+| key))

            AdvanceData(updatedShortlist, updatedProbed, hasNext = true)
        }
      }
    }

    def iterate(collected: SortedSet[Contact], probed: Set[String], data: Stream[SortedSet[Contact]]): StateT[F, RoutingTable, Seq[Contact]] =
      if (data.isEmpty) StateT.pure[F, RoutingTable, Seq[Contact]](collected.toSeq)
      else {
        val d #:: tail = data
        advance(d, probed).flatMap { updatedData =>
          if (!updatedData.hasNext) {
            iterate((collected ++ updatedData.shortlist).take(neighbors), updatedData.probed, tail)
          }
          else iterate(collected, updatedData.probed, tail append Stream(updatedData.shortlist))
        }
      }

    table[F].flatMap { rt =>
      val shortlistEmpty = SortedSet.empty[Contact]

      // Perform local lookup
      val (_, closestSeq0) = lookup[Id](key).run(rt)

      // We perform lookup on $parallelism disjoint paths
      // To ensure paths are disjoint, we keep the sole set of visited contacts
      // To synchronize the set, we iterate over $parallelism distinct shortlists
      iterate(shortlistEmpty ++ closestSeq0.take(parallelism), Set.empty, closestSeq0.take(parallelism).map(shortlistEmpty + _))
    }.map(_.take(neighbors))
  }

}