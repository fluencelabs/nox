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
import cats.effect.concurrent.MVar
import cats.effect.{Concurrent, Resource}
import cats.syntax.flatMap._
import cats.syntax.functor._

import scala.language.higherKinds

/**
 * Sink and source for control events
 * @param changePeersRef Holds a list of ChangePeer events
 * @param stopRef MVar holding stop signal
 * @tparam F Effect
 */
class ControlSignals[F[_]: FlatMap] private (
  private val changePeersRef: MVar[F, List[ChangePeer]],
  private val stopRef: MVar[F, Unit]
) {

  /**
   * Add a new ChangePeer event
   * @param change ChangePeer event
   */
  def changePeer(change: ChangePeer): F[Unit] =
    for {
      changes <- changePeersRef.take
      _ <- changePeersRef.put(changes :+ change)
    } yield ()

  // Retrieve list of current ChangePeer events
  val changePeers: Resource[F, List[ChangePeer]] =
    Resource.make(changePeersRef.tryTake.map(_.toList.flatten))(_ => changePeersRef.tryPut(Nil).void)

  // Will evaluate once ControlSignals is stopped
  val stop: F[Unit] =
    stopRef.take
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
        changePeersRef ← MVar[F].of[List[ChangePeer]](Nil)
        stopRef ← MVar.empty[F, Unit]
        instance = new ControlSignals[F](changePeersRef, stopRef)
      } yield instance
    )(_.stopRef.put(()))
}
