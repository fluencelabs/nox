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
import cats.effect.{Clock, LiftIO}
import fluence.kad.JoinError
import fluence.kad.contact.ContactAccess
import fluence.kad.protocol.{Key, Node}
import fluence.kad.state.RoutingState
import fluence.log.Log

import scala.language.higherKinds

trait IterativeRouting[F[_], C] {

  def nodeKey: Key

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
  def lookupIterative(
    key: Key,
    neighbors: Int,
    parallelism: Int
  )(implicit log: Log[F]): F[Seq[Node[C]]]

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
  def callIterative[E, A](
    key: Key,
    fn: Node[C] ⇒ EitherT[F, E, A],
    numToCollect: Int,
    parallelism: Int,
    maxNumOfCalls: Int,
    isIdempotentFn: Boolean
  )(implicit log: Log[F]): F[Vector[(Node[C], A)]]

  /**
   * Joins network with known peers
   *
   * @param peers         List of known peer contacts (assuming that Kademlia ID is unknown)
   * @param neighbors How many nodes to lookupIterative for each peer
   * @param parallelism   Parallelism factor to perform self-[[lookupIterative()]] in case of successful join
   * @return F[Unit], possibly a failure if were not able to join any node
   */
  def join(
    peers: Seq[C],
    neighbors: Int,
    parallelism: Int
  )(implicit log: Log[F]): EitherT[F, JoinError, Unit]
}

object IterativeRouting {

  def apply[F[_]: Monad: Clock: LiftIO: Parallel, P[_], C](
    localRouting: LocalRouting[F, C],
    routingState: RoutingState[F, C]
  )(implicit ca: ContactAccess[F, C]): IterativeRouting[F, C] =
    new IterativeRoutingImpl[F, P, C](localRouting, routingState)

}
