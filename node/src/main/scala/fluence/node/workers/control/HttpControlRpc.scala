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
import cats.data.EitherT
import cats.effect.Sync
import com.softwaremill.sttp.circe._
import com.softwaremill.sttp.{SttpBackend, sttp, _}
import fluence.node.workers.status.{HttpCheckFailed, HttpCheckStatus, HttpStatus}
import fluence.statemachine.control.{DropPeer, GetStatus, Stop}
import io.circe.Encoder
import scodec.bits.ByteVector
import cats.syntax.functor._
import cats.syntax.flatMap._

import scala.language.higherKinds

/**
 * Implements ControlRPC using JSON over HTTP
 *
 * @param hostname Hostname to send requests
 * @param port Port to send requests
 */
class HttpControlRpc[F[_]: Sync](hostname: String, port: Short)(
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
        .fromEither(rawResponse)
        .leftMap(msg => new Exception(s"Error sending $request: $msg"): Throwable)
    } yield response
  }

  override def dropPeer(key: ByteVector): F[Unit] =
    // TODO handle errors properly
    send(DropPeer(key), "dropPeer").void.value.flatMap(Sync[F].fromEither)

  override val status: F[HttpStatus[Unit]] =
    send(GetStatus(), "status").value.map {
      case Right(_) ⇒ HttpCheckStatus(())
      case Left(err) ⇒ HttpCheckFailed(err)
    }

  override val stop: F[Unit] =
    // TODO handle errors properly
    send(Stop(), "stop").void.value.flatMap(Sync[F].fromEither)
}
