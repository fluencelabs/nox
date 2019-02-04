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
import fs2.Sink
import fs2.concurrent.Queue
import io.circe.Decoder
import io.circe.generic.semiauto.deriveDecoder

import scala.language.higherKinds

class ControlSignals[F[_]: FlatMap] private (
  changePeersRef: MVar[F, List[ChangePeer]],
  incomingSignals: Queue[F, ControlSignal],
  stopRef: MVar[F, Unit]
) {

  incomingSignals.dequeue.evalMap {
    case c: ChangePeer => changePeer(c)
  }

  private def changePeer(change: ChangePeer): F[Unit] =
    for {
      changes <- changePeersRef.take
      _ <- changePeersRef.put(changes :+ change)
    } yield ()

  val changePeers: Resource[F, List[ChangePeer]] =
    Resource.make(changePeersRef.tryTake.map(_.toList.flatten))(_ => changePeersRef.tryPut(Nil).void)

  val stop: F[Unit] =
    stopRef.take

  val signal: Sink[F, ControlSignal] = incomingSignals.enqueue
}

sealed trait ControlSignal
// A signal to change a voting power of the specified Tendermint validator. Voting power zero votes to remove.
// Represents a Tendermint's ValidatorUpdate command
// see https://github.com/tendermint/tendermint/blob/master/docs/spec/abci/abci.md#validatorupdate
case class ChangePeer(keyType: String, validatorKey: Array[Byte], votePower: Long) extends ControlSignal

object ChangePeer {
  implicit val dec: Decoder[ChangePeer] = deriveDecoder[ChangePeer]
}

object ControlSignals {

  def apply[F[_]: Concurrent]: F[ControlSignals[F]] =
    for {
      changePeersRef ← MVar[F].of[List[ChangePeer]](Nil)
      incomingSignals <- Queue.unbounded[F, ControlSignal]
      stopRef ← MVar.empty[F, Unit]
    } yield new ControlSignals[F](changePeersRef, incomingSignals, stopRef)
}
