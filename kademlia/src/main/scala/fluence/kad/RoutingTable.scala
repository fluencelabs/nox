package fluence.kad

import cats.{Applicative, Id, Monad, MonadError}
import cats.data.StateT
import cats.syntax.monoid._
import cats.syntax.order._
import cats.syntax.applicative._
import cats.syntax.show._
import cats.syntax.functor._

import scala.collection.immutable.SortedSet
import scala.language.higherKinds

case class RoutingTable private(nodeId: Key, buckets: IndexedSeq[Bucket])

object RoutingTable {

  def apply(nodeId: Key, maxBucketSize: Int) =
    new RoutingTable(nodeId, Vector.fill(Key.BitLength)(Bucket(maxBucketSize)))

  /**
    * Just returns the table with given X effect
    *
    * @tparam X StateT effect
    * @return
    */
  def table[X[_] : Applicative]: StateT[X, RoutingTable, RoutingTable] =
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
    for {
    // Get routing table
      rt <- table[X]

      // Calculate bucket's index
      idx = (contact.key |+| rt.nodeId).zerosPrefixLen

      // Update bucket, performing ping if necessary
      updatedBucketUnit <- StateT.lift(
        Bucket
          .update[X](contact, ping)
          .run(rt.buckets(idx))
      )

      // Save updated bucket to routing table
      _ <- StateT.set(rt.copy(buckets = rt.buckets.updated(idx, updatedBucketUnit._1)))
    } yield ()

  /**
    * Tries to route a key to a Contact, if it's known locally
    *
    * @param key Key to lookup
    * @tparam X StateT effect
    * @return
    */
  def find[X[_] : Monad](key: Key): StateT[X, RoutingTable, Option[Contact]] =
    table[X].flatMapF{rt =>
      if(rt.nodeId === key) Option.empty[Contact].pure[X]
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
  def lookup[X[_] : Monad](key: Key, neighbors: Int): StateT[X, RoutingTable, Seq[Contact]] =
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
          .sortWith((l, r) => (l.key |+| key) > (r.key |+| key))
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
    * @param key Key to find neighbors for
    * @param neighbors A number of contacts to return
    * @param parallelism A number of requests performed in parallel
    * @param ping Function to perform pings, when updating routing table with newly seen nodes
    * @param lookupRemote Function to perform request to remote contact
    * @param ME Monad Error for StateT effect
    * @tparam X StateT effect
    * @return
    */
  def lookupIterative[X[_]](
                             key: Key,
                             neighbors: Int,

                             parallelism: Int,

                             ping: Contact => X[Contact],
                             lookupRemote: (Contact, Key, Int) => X[Seq[Contact]]

                           )(implicit ME: MonadError[X, Throwable]): StateT[X, RoutingTable, Seq[Contact]] = {
    val probed = Set.empty[String] // probed node ids
    val shortlist = SortedSet.empty[Contact]((x, y) => (x.key |+| key) compare (y.key |+| key))
    val queue = SortedSet.empty[Contact]((x, y) => (x.key |+| key) compare (y.key |+| key))

    table[X].flatMap {rt =>
      val (_, sq0) = lookup[Id](key, neighbors).run(rt) // local lookup

      val sl = shortlist ++ sq0 // to shortlist
      val pbd = probed ++ sq0.map(_.key.show) // we've seen these nodes



      sl.take(parallelism).map{c =>

        lookupRemote(c, key, neighbors).map {cnts =>
          // add c to seen
          // add unseen cnts to shortlist
          //

          // if either no closer nodes are returned,
          // or all nodes are already queried ($neighbors closest nodes are all visited),
          // stop iterations
          sl ++ cnts
        }

      }

      ???
    }


    // get _neighbors_ local closest nodes
    // for _parallelism_ closest nodes, fetch _neighbors_ closest nodes
   ???
  }

}