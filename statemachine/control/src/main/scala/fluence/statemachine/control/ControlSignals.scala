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

package fluence.statemachine.control
import cats.FlatMap
import cats.effect.concurrent.{Deferred, MVar}
import cats.effect.{Concurrent, Resource, Sync}
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.syntax.applicativeError._

import scala.language.higherKinds

/**
 * Sink and source for control events
 * @param dropPeersRef Holds a list of DropPeer events
 * @param stopRef Deferred holding stop signal, completed when the worker should stop
 * @tparam F Effect
 */
class ControlSignals[F[_]: FlatMap] private (
  private val dropPeersRef: MVar[F, List[DropPeer]],
  private val stopRef: Deferred[F, Unit]
) {

  /**
   * Add a new DropPeer event
   */
  private[control] def dropPeer(drop: DropPeer): F[Unit] =
    for {
      changes <- dropPeersRef.take
      _ <- dropPeersRef.put(changes :+ drop)
    } yield ()

  /**
   * Retrieve list of current DropPeer events
   */
  val dropPeers: Resource[F, List[DropPeer]] =
    Resource.make(dropPeersRef.tryTake.map(_.toList.flatten))(_ => dropPeersRef.tryPut(Nil).void)

  /**
   * Orders the worker to stop
   */
  private[control] def stopWorker(): F[Unit] = stopRef.complete(())

  /**
   * Will evaluate once the worker should stop
   */
  val stop: F[Unit] = stopRef.get
}

object ControlSignals {

  /**
   * Create a resource holding ControlSignals. Stop ControlSignals after resource is used.
   * @tparam F Effect
   * @return Resource holding a ControlSignals instance
   */
  def apply[F[_]: Concurrent](): Resource[F, ControlSignals[F]] =
    Resource.make(
      for {
        dropPeersRef ← MVar[F].of[List[DropPeer]](Nil)
        stopRef ← Deferred[F, Unit]
        instance = new ControlSignals[F](dropPeersRef, stopRef)
      } yield instance
    ) { s =>
      Sync[F].suspend(
        // Tell worker to stop if ControlSignals is dropped
        s.stopRef.complete(()).attempt.void
      )
    }
}
