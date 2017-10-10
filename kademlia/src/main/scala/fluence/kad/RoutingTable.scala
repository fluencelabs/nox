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

case class RoutingTable private (nodeId: Key, buckets: IndexedSeq[Bucket], size: Int = 0)

object RoutingTable {

  implicit def show(implicit bs: Show[Bucket], ks: Show[Key]): Show[RoutingTable] =
    rt => s"RoutingTable(${ks.show(rt.nodeId)})" +
      rt.buckets.zipWithIndex.filter(_._1.nonEmpty).map{
        case (b, i) => s"$i(${Integer.toBinaryString(i)}):${bs.show(b)}"
      }.mkString(":\n", "\n", "")

  def apply(nodeId: Key, maxBucketSize: Int) =
    new RoutingTable(nodeId, Vector.fill(Key.BitLength)(Bucket(maxBucketSize)))

  /**
   * Just returns the table with given X effect
   *
   * @tparam X StateT effect
   * @return
   */
  def table[X[_]: Applicative]: StateT[X, RoutingTable, RoutingTable] =
    StateT.get

  /**
   * Locates the bucket responsible for given contact, and updates it using given ping function
   *
   * @param contact Contact to update
   * @param ping    Function that pings the contact to check if it's alive
   * @param ME      Monad error instance
   * @tparam X StateT effect
   * @return
   */
  def update[X[_]](contact: Contact, ping: Contact => X[Contact])(implicit ME: MonadError[X, Throwable]): StateT[X, RoutingTable, Unit] =
    table[X].flatMap { rt =>
      if (rt.nodeId === contact.key) StateT.pure(())
      else {
        val idx = (contact.key |+| rt.nodeId).zerosPrefixLen

        val bucket = rt.buckets(idx)

        for {
          // Update bucket, performing ping if necessary
          updatedBucketUnit <- StateT.lift(
            Bucket
              .update[X](contact, ping)
              .run(bucket)
          )

          // Save updated bucket to routing table
          _ <- StateT.set(rt.copy(
            buckets = rt.buckets.updated(idx, updatedBucketUnit._1),
            size = rt.size - bucket.size + updatedBucketUnit._1.size)
          )
        } yield ()
      }
    }

  /**
   * Tries to route a key to a Contact, if it's known locally
   *
   * @param key Key to lookup
   * @tparam X StateT effect
   * @return
   */
  def find[X[_]: Monad](key: Key): StateT[X, RoutingTable, Option[Contact]] =
    table[X].flatMapF { rt =>
      if (rt.nodeId === key) Option.empty[Contact].pure[X]
      else {
        val idx = (key |+| rt.nodeId).zerosPrefixLen
        Bucket.find[X](key).run(rt.buckets(idx)).map(_._2)
      }
    }

  /**
   * Performs local lookup for the key, returning a number of closest known nodes to it
   *
   * @param key       Key to lookup
   * @param neighbors Number of closest contacts to return
   * @tparam X StateT effect
   * @return
   */
  def lookup[X[_]: Monad](key: Key, neighbors: Int): StateT[X, RoutingTable, Seq[Contact]] =
    table[X].flatMap { rt =>

      // Base index: nodes as far from this one as the target key is
      val idx = (rt.nodeId |+| key).zerosPrefixLen

      // Diverging stream of indices, going left (far from current node) then right (closer), like 5 4 6 3 7 ...
      StateT pure Stream(idx)
        .filter(_ < Key.BitLength) // In case current node is given, this will remove IndexOutOfBoundsException
        .append(Stream.from(1).takeWhile(i => idx + i < Key.BitLength || idx - i >= 0).flatMap { i =>
        (if (idx - i >= 0) Stream(idx - i) else Stream.empty) append
          (if (idx + i < Key.BitLength) Stream(idx + i) else Stream.empty)
      })
        .flatMap(
          // Take contacts from the bucket, and sort them
          rt.buckets(_)
            .contacts
            .sortWith((l, r) => (l.key |+| key) < (r.key |+| key))
            .toStream
        ).take(neighbors) // Stream is a Seq
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
   * @tparam X StateT effect
   * @return
   */
  def lookupIterative[X[_]](
    key: Key,
    neighbors: Int,

    parallelism: Int,

    ping: Contact => X[Contact],
    lookupRemote: (Contact, Key, Int) => X[Seq[Contact]],

    log: Any => Unit = _ => ()

  )(implicit ME: MonadError[X, Throwable], sk: Show[Key]): StateT[X, RoutingTable, Seq[Contact]] = {
    // Import for Traverse
    import cats.instances.list._

    // As we see nodes, update routing table
    def updateTable(pending: List[Contact], checked: List[Contact] = Nil): StateT[X, RoutingTable, List[Contact]] =
      pending match {
        case a :: tail =>
          update(a, ping).flatMap(_ =>
            updateTable(tail, a :: checked)
          )

        case Nil =>
          StateT.pure(checked)
      }

    // Query $parallelism more nodes, looking for better results
    def iterate(shortlist: SortedSet[Contact], probed: Set[String]): StateT[X, RoutingTable, Seq[Contact]] = {
      // Take $parallelism unvisited nodes to perform lookups on
      val handle = shortlist.filter(c => !probed(c.key.show)).take(parallelism).toList

      log("\nITERATE W/SHORTLIST "+shortlist.toVector.map(_.key.show))
      log("Handle: "+handle.map(_.key.show))

      // If handle is empty, return
      if (handle.isEmpty || shortlist.isEmpty) {
        StateT.pure[X, RoutingTable, Seq[Contact]](shortlist.toSeq)
      } else {

        // The closest node -- we're trying to improve this result
        //val closest = shortlist.head

        // We're going to probe handled, and want to filter them out
        val updatedProbed = probed ++ handle.map(_.key.show)

        // Fetch remote lookups into X; filter previously seen nodes
        val remote0X = Traverse[List].sequence(handle.map { c =>
          lookupRemote(c, key, neighbors)
        }).map[List[Contact]](
          _.flatten
            .filterNot(c => updatedProbed(c.key.show)) // Filter away already seen nodes
        )

        StateT.lift[X, RoutingTable, List[Contact]](remote0X)
          .flatMap(updateTable(_)) // Update routing table
          .flatMap {
            remotes =>
              log("Got remotes: "+remotes.map(_.key.show))
              val updatedShortlist = shortlist ++ remotes.filter(c => shortlist.size < neighbors || (c.key |+| key) < (shortlist.head.key |+| key))

              iterate(updatedShortlist, updatedProbed) // Iterate
          }
      }
    }

    table[X].flatMap { rt =>
      val shortlistEmpty = SortedSet.empty[Contact]((x, y) => (x.key |+| key) compare (y.key |+| key))

      // Perform local lookup
      val (_, closestSeq0) = lookup[Id](key, parallelism).run(rt)

      // Perform first iteration
      iterate(shortlistEmpty ++ closestSeq0, Set.empty)
    }.map(_.take(neighbors))
  }

}