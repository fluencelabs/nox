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

package fluence.node

import cats.data.EitherT
import fluence.effects.tendermint.block.history.Receipt
import fluence.node.workers.control.{ControlRpc, ControlRpcError}
import fluence.node.workers.status.HttpStatus
import fluence.statemachine.control.ControlStatus
import scodec.bits.ByteVector

import scala.language.higherKinds

trait TestControlRpc[F[_]] extends ControlRpc[F] {

  /**
   * Request worker to send a vote to Tendermint for removal of a validator
   *
   * @param key Public key of the Tendermint validator
   */
  override def dropPeer(key: ByteVector): EitherT[F, ControlRpcError, Unit] =
    throw new NotImplementedError("def dropPeer")

  /**
   * Request current worker status
   *
   * @return Currently if method returned without an error, worker is considered to be healthy
   */
  override def status: F[HttpStatus[ControlStatus]] = throw new NotImplementedError("def status")

  /**
   * Requests worker to stop
   */
  override def stop: EitherT[F, ControlRpcError, Unit] = throw new NotImplementedError("def stop")

  /**
   * Send block manifest receipt, so state machine can use it for app hash calculation
   */
  override def sendBlockReceipt(receipt: Receipt): EitherT[F, ControlRpcError, Unit] =
    throw new NotImplementedError("def sendBlockReceipt")

  /**
   * Retrieves vm hash from state machine, required for block manifest uploading
   */
  override def getVmHash(height: Long): EitherT[F, ControlRpcError, ByteVector] =
    throw new NotImplementedError(s"def getVmHash $height")
}
