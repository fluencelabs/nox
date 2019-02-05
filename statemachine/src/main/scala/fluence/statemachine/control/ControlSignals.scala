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
import cats.Functor
import cats.effect.Concurrent
import cats.effect.concurrent.MVar
import cats.syntax.flatMap._
import cats.syntax.functor._
import scodec.bits.ByteVector

import scala.language.higherKinds

class ControlSignals[F[_]: Functor] private (
  changePeersRef: MVar[F, List[ChangePeer]],
  stopRef: MVar[F, Unit]
) {

  val changePeers: F[List[ChangePeer]] =
    changePeersRef.tryTake.map(_.toList.flatten)

  val stop: F[Unit] =
    stopRef.take
}

object ControlSignals {
  case class ChangePeer(`type`: String, data: ByteVector, power: Long)

  def apply[F[_]: Concurrent]: F[ControlSignals[F]] =
    for {
      changePeersRef ← MVar.empty[F, List[ChangePeer]]
      stopRef ← MVar.empty[F, Unit]
    } yield new ControlSignals[F](changePeersRef, stopRef)
}
