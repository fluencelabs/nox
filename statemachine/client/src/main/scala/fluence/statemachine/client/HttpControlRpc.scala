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

package fluence.statemachine.client

import cats.Monad
import cats.data.EitherT
import cats.syntax.functor._
import com.softwaremill.sttp.circe._
import com.softwaremill.sttp.{sttp, _}
import fluence.effects.sttp.{SttpEffect, SttpError}
import fluence.effects.sttp.syntax._
import fluence.statemachine.api.StateMachineStatus
import fluence.statemachine.api.signals._
import io.circe.{Decoder, Encoder}
import io.circe.parser.decode
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * Implements ControlRPC using JSON over HTTP
 *
 * @param hostname Hostname to send requests
 * @param port Port to send requests
 */
class HttpControlRpc[F[_]: Monad: SttpEffect](hostname: String, port: Short) extends ControlRpc[F] {

  // TODO move this code somewhere
  private implicit val decbc: Decoder[ByteVector] =
    Decoder.decodeString.flatMap(
      ByteVector.fromHex(_).fold(Decoder.failedWithMessage[ByteVector]("Not a hex"))(Decoder.const)
    )

  /**
   * Send a serializable request to the worker's control endpoint
   *
   * @param request Control RPC request
   * @param path Control RPC path
   */
  private def send[Req: Encoder](request: Req, path: String): EitherT[F, SttpError, Response[String]] =
    sttp
      .body(request)
      .post(uri"http://$hostname:$port/control/$path")
      .send()

  override def dropPeer(key: ByteVector): EitherT[F, ControlRpcError, Unit] =
    // TODO handle errors properly
    send(DropPeer(key), "dropPeer").void.leftMap(DropPeerError(key, _))

  override val status: EitherT[F, SttpError, StateMachineStatus] =
    send(GetStatus(), "status").decodeBody(decode[StateMachineStatus](_))

  override val stop: EitherT[F, ControlRpcError, Unit] =
    send(Stop(), "stop").void.leftMap(WorkerStatusError)

  override def sendBlockReceipt(receipt: BlockReceipt): EitherT[F, ControlRpcError, Unit] =
    send(receipt, "blockReceipt").void
      .leftMap(SendBlockReceiptError(receipt, _))

  override def getVmHash(height: Long): EitherT[F, ControlRpcError, ByteVector] = {

    send(GetVmHash(height), "vmHash")
      .decodeBody(decode[ByteVector](_))
      .leftMap(GetVmHashError)
  }
}
