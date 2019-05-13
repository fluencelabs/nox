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

import java.util.concurrent.TimeUnit

import cats.Traverse
import cats.data.EitherT
import cats.effect.{Concurrent, Fiber, Sync, Timer}
import cats.effect.concurrent.MVar
import fluence.kad.JoinError
import fluence.kad.protocol.{Key, Node}
import cats.syntax.applicative._
import cats.syntax.flatMap._
import cats.syntax.apply._
import cats.syntax.functor._
import cats.instances.stream._
import cats.syntax.monoid._

import scala.concurrent.duration._
import scala.util.Random
import scala.language.higherKinds

/**
 * Wraps [[IterativeRouting]] and provides routing table refreshing:
 * - For each bucket, once in a refreshTimeout, perform a [[lookupIterative]] for a random key from that bucket
 * - If there was any user-triggered lookupIterative for a bucket, postpone the scheduled refreshing job accordingly
 *
 * @param routing Actual routing is delegated there
 * @param scheduleRefresh For the given bucketId, schedule a refresh job and return the corresponding fiber
 * @param refreshingFibers State of refresh fibers // TODO is it optimal? won't it cause deadlocks? maybe map of mvars is better then mvar of map?
 * @tparam F Effect
 * @tparam C Contact
 */
private class RefreshingIterativeRouting[F[_]: Concurrent, C](
  routing: IterativeRouting[F, C],
  scheduleRefresh: Int ⇒ F[Fiber[F, Unit]],
  refreshingFibers: Map[Int, MVar[F, Fiber[F, Unit]]]
) extends IterativeRouting[F, C] {

  override def nodeKey: Key = routing.nodeKey

  /**
   * Called each time when any *Iterative method is called by user, reschedules the refreshing job for the bucket
   *
   * @param key Touched key
   */
  private def rescheduleRefresh(key: Key): F[Unit] = {
    val distance = key.distanceTo(nodeKey)

    val mFiber = refreshingFibers(distance)

    mFiber.tryTake.flatMap {
      case Some(fiber) ⇒
        // Do not do what was scheduled, schedule refresh instead, save schedule fiber
        (fiber.cancel >> scheduleRefresh(distance)).map(mFiber.put)
      case None ⇒
        // It's in progress of re-scheduling, do nothing
        ().pure[F]
    }
  }

  override def lookupIterative(key: Key, neighbors: Int, parallelism: Int): F[Seq[Node[C]]] =
    rescheduleRefresh(key) *> routing.lookupIterative(key, neighbors, parallelism)

  override def callIterative[E, A](
    key: Key,
    fn: Node[C] ⇒ EitherT[F, E, A],
    numToCollect: Int,
    parallelism: Int,
    maxNumOfCalls: Int,
    isIdempotentFn: Boolean
  ): F[Vector[(Node[C], A)]] =
    rescheduleRefresh(key) *> routing.callIterative(key, fn, numToCollect, parallelism, maxNumOfCalls, isIdempotentFn)

  override def join(peers: Seq[C], numberOfNodes: Int, parallelism: Int): EitherT[F, JoinError, Unit] =
    routing.join(peers, numberOfNodes, parallelism) <* EitherT.liftF(rescheduleRefresh(nodeKey))
}

object RefreshingIterativeRouting {

  /**
   *
   * @param iterativeRouting [[IterativeRouting]] implementation to delegate calls to
   * @param refreshTimeout Refresh timeouts
   * @param refreshNeighbors Number of nodes to lookup on refresh
   * @param parallelism Parallelism factor, should be taken from Kademlia config
   * @tparam F Effect
   * @tparam C Contact
   * @return Enhanced [[IterativeRouting]]
   */
  def apply[F[_]: Concurrent: Timer, C](
    iterativeRouting: IterativeRouting[F, C],
    refreshTimeout: FiniteDuration,
    refreshNeighbors: Int,
    parallelism: Int
  ): F[IterativeRouting[F, C]] = {
    val nodeKey = iterativeRouting.nodeKey

    (
      // Bootstrap the Random that will be used to generate Keys and jitters
      Timer[F].clock.realTime(TimeUnit.MILLISECONDS).map(new Random(_)),
      // Prepare a map of empty MVars, where scheduled refreshing jobs will be stored
      Traverse[Stream]
        .traverse(Stream.range[Int](0, Key.BitLength - 1))(
          idx ⇒ MVar.empty[F, Fiber[F, Unit]].map(idx -> _)
        )
        .map(_.toMap)
    ).tupled.flatMap {
      case (rnd, refreshFibers) ⇒
        // Generate new timeout, sleep
        val sleep = Sync[F].delay(refreshTimeout + rnd.nextInt.abs.millis) >>= Timer[F].sleep

        // Lookup the bucket iteratively
        def refresh(idx: Int) =
          iterativeRouting
            .lookupIterative(nodeKey.randomDistantKey(idx, rnd), refreshNeighbors, parallelism)
            .void

        // Drop current fiber out from MVar, because it refers to current execution flow
        // Schedule new refresh, save its fiber
        def reschedule(idx: Int) =
          refreshFibers(idx).take >> scheduleRefresh(idx) >>= refreshFibers(idx).put

        // Schedule refreshing job for a bucket
        def scheduleRefresh(idx: Int): F[Fiber[F, Unit]] =
          Concurrent[F].start(
            sleep >> refresh(idx) >> reschedule(idx)
          )

        val routing = new RefreshingIterativeRouting(
          iterativeRouting,
          scheduleRefresh,
          refreshFibers
        )

        // All MVars are empty yet, so just put fibers there
        Traverse[Stream].traverse(Stream.range[Int](0, Key.BitLength - 1))(
          idx ⇒ scheduleRefresh(idx) >>= refreshFibers(idx).put
        ) as routing
    }
  }
}
