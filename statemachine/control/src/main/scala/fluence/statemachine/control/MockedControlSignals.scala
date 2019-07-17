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
import fluence.effects.tendermint.block.history.Receipt
import scodec.bits.ByteVector

// TODO: move to tests

class MockedControlSignals extends ControlSignals[IO] {
  @volatile private var height: Int = 0

  override val dropPeers: Resource[IO, Set[DropPeer]] = Resource.pure(Set.empty)
  override val stop: IO[Unit] = IO.unit
  override val receipt: IO[BlockReceipt] = {
    height += 1
    IO.pure(BlockReceipt(Receipt(height, ByteVector.empty), ReceiptType.New))
  }
  override def dropPeer(drop: DropPeer): IO[Unit] = IO.unit
  override def stopWorker(): IO[Unit] = IO.unit
  override def putReceipt(receipt: BlockReceipt): IO[Unit] = IO.unit
  override def enqueueVmHash(height: Long, hash: ByteVector): IO[Unit] = IO.unit
  override def getVmHash(height: Long): IO[VmHash] = IO.pure(VmHash(-1, ByteVector.empty))
}
