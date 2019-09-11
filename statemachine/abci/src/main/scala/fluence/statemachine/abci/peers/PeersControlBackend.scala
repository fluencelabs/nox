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

package fluence.statemachine.abci.peers

import cats.Monad
import cats.data.EitherT
import cats.effect.{Concurrent, Resource}
import cats.effect.concurrent.MVar
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.effects.EffectError
import fluence.log.Log
import fluence.statemachine.api.command.PeersControl
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 *
 * @param dropPeersRef Holds a set of DropPeer events. NOTE: since Tendermint 0.30.0 Validator set updates must be unique by pub key
 * @tparam F Effect
 */
class PeersControlBackend[F[_]: Monad](
  private val dropPeersRef: MVar[F, Set[DropPeer]]
) extends PeersControl[F] {
  override def dropPeer(validatorKey: ByteVector)(implicit log: Log[F]): EitherT[F, EffectError, Unit] =
    EitherT.right(for {
      changes <- dropPeersRef.take
      _ <- dropPeersRef.put(changes + DropPeer(validatorKey))
    } yield ())

  /**
   * Move list of current DropPeer events from ControlSignals to call-site
   * dropPeersRef is emptied on resource's acquisition, and filled with Nil after resource is used
   * Using Resource this way guarantees exclusive access to data
   *
   * @return Resource with List of DropPeer signals
   */
  private[statemachine] val dropPeers: Resource[F, Set[DropPeer]] =
    Resource.make(
      dropPeersRef.tryTake.map(_.getOrElse(Set.empty))
    )(_ => dropPeersRef.tryPut(Set.empty).void)

}

object PeersControlBackend {

  def apply[F[_]: Concurrent]: F[PeersControlBackend[F]] =
    MVar[F].of[Set[DropPeer]](Set.empty).map(new PeersControlBackend[F](_))
}
