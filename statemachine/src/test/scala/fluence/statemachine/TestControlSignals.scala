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

package fluence.statemachine

import cats.effect.{IO, Resource}
import fluence.statemachine.control.signals.{BlockReceipt, ControlSignals, DropPeer}
import fluence.statemachine.control.VmHash
import scodec.bits.ByteVector

trait TestControlSignals extends ControlSignals[IO] {
  override val dropPeers: Resource[IO, Set[DropPeer]] =
    Resource.liftF(IO(throw new NotImplementedError("val dropPeers")))
  override val stop: IO[Unit] = IO(throw new NotImplementedError("val stop"))
  override def getReceipt(height: Long): IO[BlockReceipt] = IO(throw new NotImplementedError(s"def receipt $height"))
  override def dropPeer(drop: DropPeer): IO[Unit] = IO(throw new NotImplementedError("def dropPeer"))
  override def stopWorker(): IO[Unit] = IO(throw new NotImplementedError("def stopWorker"))
  override def enqueueReceipt(receipt: BlockReceipt): IO[Unit] = IO(throw new NotImplementedError("def putReceipt"))
  override def enqueueVmHash(height: Long, hash: ByteVector): IO[Unit] =
    IO(throw new NotImplementedError("def enqueueVmHash"))
  override def getVmHash(height: Long): IO[VmHash] = IO(throw new NotImplementedError("def getVmHash"))
}
