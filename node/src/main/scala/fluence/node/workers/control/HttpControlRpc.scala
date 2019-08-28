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

package fluence.node.workers.control

import cats.Monad
import cats.data.EitherT
import cats.syntax.flatMap._
import cats.syntax.functor._
import com.softwaremill.sttp.circe._
import com.softwaremill.sttp.{SttpBackend, sttp, _}
import fluence.effects.tendermint.block.history.{helpers, Receipt}
import fluence.node.workers.status.{HttpCheckFailed, HttpCheckStatus, HttpStatus}
import fluence.statemachine.control.signals.{BlockReceipt, DropPeer, GetStatus, GetVmHash, Stop}
import fluence.statemachine.control.ControlStatus
import io.circe.Encoder
import io.circe.parser.parse
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * Implements ControlRPC using JSON over HTTP
 *
 * @param hostname Hostname to send requests
 * @param port Port to send requests
 */
class HttpControlRpc[F[_]: Monad](hostname: String, port: Short)(
  implicit s: SttpBackend[EitherT[F, Throwable, ?], Nothing]
) extends ControlRpc[F] {

  /**
   * Send a serializable request to the worker's control endpoint
   *
   * @param request Control RPC request
   * @param path Control RPC path
   */
  private def send[Req: Encoder](request: Req, path: String): EitherT[F, Throwable, String] = {
    for {
      rawResponse <- sttp
        .body(request)
        .post(uri"http://$hostname:$port/control/$path")
        .send()
        .map(_.body)
      response <- EitherT
        .fromEither[F](rawResponse)
        .leftMap(msg => new Exception(s"Error sending $request $path: $msg"): Throwable)
    } yield response
  }

  override def dropPeer(key: ByteVector): EitherT[F, ControlRpcError, Unit] =
    // TODO handle errors properly
    send(DropPeer(key), "dropPeer").void.leftMap(DropPeerError(key, _))

  override val status: F[HttpStatus[ControlStatus]] =
    send(GetStatus(), "status").subflatMap(parse).subflatMap(_.as[ControlStatus]).value.map {
      case Right(st) ⇒ HttpCheckStatus(st)
      case Left(err) ⇒ HttpCheckFailed(err)
    }

  override val stop: EitherT[F, ControlRpcError, Unit] =
    send(Stop(), "stop").void.leftMap(WorkerStatusError)

  override def sendBlockReceipt(receipt: Receipt): EitherT[F, ControlRpcError, Unit] =
    send(BlockReceipt(receipt), "blockReceipt").void.leftMap(SendBlockReceiptError(receipt, _))

  override def getVmHash(height: Long): EitherT[F, ControlRpcError, ByteVector] = {
    import io.circe.parser._
    import helpers.ByteVectorJsonCodec._

    send(GetVmHash(height), "vmHash")
      .subflatMap(parse(_).flatMap(_.as[ByteVector]))
      .leftMap(GetVmHashError)
  }
}
