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

import cats.effect.{IO, Resource}
import fluence.statemachine.api.StateHash
import fluence.statemachine.api.signals.{BlockReceipt, DropPeer}
import fluence.statemachine.control.signals.ControlSignals
import scodec.bits.ByteVector

class MockedControlSignals extends ControlSignals[IO] {
  override val dropPeers: Resource[IO, Set[DropPeer]] = Resource.pure(Set.empty)
  override val stop: IO[Unit] = IO.unit
  override def getReceipt(height: Long): IO[BlockReceipt] = {
    IO.pure(BlockReceipt(height, ByteVector.empty))
  }

  override def dropPeer(drop: DropPeer): IO[Unit] = IO.unit
  override def stopWorker(): IO[Unit] = IO.unit
  override def enqueueReceipt(receipt: BlockReceipt): IO[Unit] = IO.unit
  override def enqueueStateHash(height: Long, hash: ByteVector): IO[Unit] = IO.unit
  override def getStateHash(height: Long): IO[StateHash] = IO.pure(StateHash(-1, ByteVector.empty))

  override def lastStateHash: IO[StateHash] = IO.pure(StateHash(-1, ByteVector.empty))
}
