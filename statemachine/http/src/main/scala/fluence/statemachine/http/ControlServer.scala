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

package fluence.statemachine.http

import cats.effect._
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.log.{Log, LogFactory}
import fluence.statemachine.api.StateMachineStatus
import fluence.statemachine.api.signals.{BlockReceipt, DropPeer, GetVmHash}
import fluence.statemachine.control.signals.ControlSignals
import io.circe.{Decoder, Encoder}
import io.circe.syntax._
import org.http4s._
import org.http4s.circe._
import org.http4s.dsl.Http4sDsl
import scodec.bits.ByteVector

import scala.language.higherKinds

object ControlServer {

  // TODO move this code somewhere
  private implicit val decbc: Decoder[ByteVector] =
    Decoder.decodeString.flatMap(
      ByteVector.fromHex(_).fold(Decoder.failedWithMessage[ByteVector]("Not a hex"))(Decoder.const)
    )

  private implicit val encbc: Encoder[ByteVector] = Encoder.encodeString.contramap(_.toHex)

  /**
   * Run http json rpc server
   *
   * @param signals Control events sink
   * @tparam F Effect
   * @return
   */
  def routes[F[_]: Concurrent: LogFactory](
    signals: ControlSignals[F],
    status: F[StateMachineStatus]
  )(implicit dsl: Http4sDsl[F]): HttpRoutes[F] = {
    import dsl._

    implicit val dpdec: EntityDecoder[F, DropPeer] = jsonOf[F, DropPeer]
    implicit val bpdec: EntityDecoder[F, BlockReceipt] = jsonOf[F, BlockReceipt]
    implicit val gvdec: EntityDecoder[F, GetVmHash] = jsonOf[F, GetVmHash]
    implicit val bvenc: EntityEncoder[F, ByteVector] = jsonEncoderOf[F, ByteVector]

    def logReq(req: Request[F]): F[Log[F]] =
      LogFactory[F]
        .init("method" -> req.method.toString(), "path" -> req.pathInfo)
        .flatTap(_.info(s"request"))
        .widen[Log[F]]

    HttpRoutes.of {
      case req @ POST -> Root / "dropPeer" =>
        for {
          implicit0(log: Log[F]) ← logReq(req)
          drop <- req.as[DropPeer]
          _ <- signals.dropPeer(drop)
          ok <- Ok()
        } yield ok

      case req @ POST -> Root / "stop" =>
        for {
          implicit0(log: Log[F]) ← logReq(req)
          _ ← signals.stopWorker()
          ok ← Ok()
        } yield ok

      case (GET | POST) -> Root / "status" =>
        status.map(_.asJson.noSpaces) >>= (json ⇒ Ok(json))

      case req @ POST -> Root / "blockReceipt" =>
        for {
          implicit0(log: Log[F]) ← logReq(req)
          receipt <- req.as[BlockReceipt]
          _ <- signals.enqueueReceipt(receipt)
          ok <- Ok()
        } yield ok

      case req @ (GET | POST) -> Root / "vmHash" =>
        for {
          implicit0(log: Log[F]) ← logReq(req)
          GetVmHash(height) <- req.as[GetVmHash]
          vmHash <- signals.getStateHash(height)
          ok <- Ok(vmHash.hash)
        } yield ok
    }
  }
}
