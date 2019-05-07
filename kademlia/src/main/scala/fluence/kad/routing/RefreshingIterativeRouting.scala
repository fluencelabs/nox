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
import cats.effect.{Concurrent, Fiber, Timer}
import cats.effect.concurrent.MVar
import fluence.kad.JoinError
import fluence.kad.protocol.{Key, Node}
import cats.syntax.apply._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.instances.stream._
import cats.syntax.monoid._

import scala.concurrent.duration.FiniteDuration
import scala.util.Random
import scala.language.higherKinds

private class RefreshingIterativeRouting[F[_]: Concurrent: Timer, C](
  nodeId: Key,
  rnd: Random,
  routing: IterativeRouting[F, C],
  refreshTimeout: FiniteDuration,
  refreshNeighbors: Int,
  parallelism: Int,
  refreshFibers: MVar[F, Map[Int, Fiber[F, Unit]]]
) extends IterativeRouting[F, C] {

  private def touchedIterative(key: Key): F[Unit] = {
    val idx = (key |+| nodeId).zerosPrefixLen

    for {
      rf ← refreshFibers.take
      _ ← rf(idx).cancel
      fiber ← Concurrent[F].start(
        Timer[F]
          .sleep(refreshTimeout) *> lookupIterative(nodeId.randomize(idx, rnd), refreshNeighbors, parallelism).void
      )
      _ ← refreshFibers.put(rf.updated(idx, fiber))
    } yield ()
  }

  override def lookupIterative(key: Key, neighbors: Int, parallelism: Int): F[Seq[Node[C]]] =
    touchedIterative(key) *> routing.lookupIterative(key, neighbors, parallelism)

  override def callIterative[E, A](
    key: Key,
    fn: Node[C] ⇒ EitherT[F, E, A],
    numToCollect: Int,
    parallelism: Int,
    maxNumOfCalls: Int,
    isIdempotentFn: Boolean
  ): F[Vector[(Node[C], A)]] =
    touchedIterative(key) *> routing.callIterative(key, fn, numToCollect, parallelism, maxNumOfCalls, isIdempotentFn)

  override def join(peers: Seq[C], numberOfNodes: Int, parallelism: Int): EitherT[F, JoinError, Unit] =
    routing.join(peers, numberOfNodes, parallelism) <* EitherT.liftF(touchedIterative(nodeId))
}

object RefreshingIterativeRouting {

  def apply[F[_]: Concurrent: Timer, C](
    nodeId: Key,
    iterativeRouting: IterativeRouting[F, C],
    refreshTimeout: FiniteDuration,
    refreshNeighbors: Int,
    parallelism: Int
  ): F[IterativeRouting[F, C]] =
    for {
      t <- Timer[F].clock.realTime(TimeUnit.MILLISECONDS)

      rnd = new Random(t)
      fibers = Stream
        .range[Int](0, Key.BitLength - 1)
        .map(_ -> Fiber[F, Unit](Concurrent[F].never, Concurrent[F].unit))
        .toMap

      refreshFibers ← MVar.of(fibers)

      routing = new RefreshingIterativeRouting(
        nodeId,
        rnd,
        iterativeRouting,
        refreshTimeout,
        refreshNeighbors,
        parallelism,
        refreshFibers
      )

      _ ← refreshFibers.take

      fibers ← Traverse[Stream].traverse(Stream.range[Int](0, Key.BitLength - 1))(
        idx ⇒
          Concurrent[F]
            .start(
              Timer[F].sleep(refreshTimeout) *> routing
                .lookupIterative(nodeId.randomize(idx, rnd), refreshNeighbors, parallelism)
                .void
            )
            .map(idx -> _)
      )

      _ ← refreshFibers.put(fibers.toMap)

    } yield routing
}
