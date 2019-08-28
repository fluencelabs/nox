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
import cats.data.Kleisli
import cats.effect._
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.effects.tendermint.block.history.helpers
import fluence.log.{Log, LogFactory}
import fluence.statemachine.control.signals.{BlockReceipt, ControlSignals, DropPeer, GetVmHash}
import io.circe.syntax._
import org.http4s._
import org.http4s.circe._
import org.http4s.dsl.Http4sDsl
import org.http4s.implicits._
import org.http4s.server.Server
import org.http4s.server.blaze._
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * Represents HTTP JSON RPC server sending requests to ControlSignals for later consumption by state machine
 *
 * @param signals Control events sink
 * @param http Http json rpc server
 * @tparam F Effect
 */
case class ControlServer[F[_]](signals: ControlSignals[F], http: Server[F])

object ControlServer {

  /** Settings for [[ControlServer]]
   *
   * @param host host to listen on
   * @param port port to listen on
   */
  case class Config(host: String, port: Short)

  /**
   * Run http json rpc server
   *
   * @param signals Control events sink
   * @tparam F Effect
   * @return
   */
  private def controlService[F[_]: Concurrent: LogFactory](
    signals: ControlSignals[F]
  )(implicit dsl: Http4sDsl[F]): Kleisli[F, Request[F], Response[F]] = {
    import dsl._
    import helpers.ByteVectorJsonCodec._

    implicit val dpdec: EntityDecoder[F, DropPeer] = jsonOf[F, DropPeer]
    implicit val bpdec: EntityDecoder[F, BlockReceipt] = jsonOf[F, BlockReceipt]
    implicit val gvdec: EntityDecoder[F, GetVmHash] = jsonOf[F, GetVmHash]
    implicit val bvenc: EntityEncoder[F, ByteVector] = jsonEncoderOf[F, ByteVector]

    def logReq(req: Request[F]): F[Log[F]] =
      LogFactory[F]
        .init("method" -> req.method.toString(), "ctrl" -> req.pathInfo)
        .flatTap(_.info(s"request"))
        .widen[Log[F]]

    val route: PartialFunction[Request[F], F[Response[F]]] = {
      case req @ POST -> Root / "control" / "dropPeer" =>
        for {
          implicit0(log: Log[F]) ← logReq(req)
          drop <- req.as[DropPeer]
          _ <- signals.dropPeer(drop)
          ok <- Ok()
        } yield ok

      case req @ POST -> Root / "control" / "stop" =>
        for {
          implicit0(log: Log[F]) ← logReq(req)
          _ ← signals.stopWorker()
          ok ← Ok()
        } yield ok

      case (GET | POST) -> Root / "control" / "status" =>
        // TODO check whether eth blocks are actually expected
        Ok(ControlStatus(expectEth = false).asJson.noSpaces)

      case req @ POST -> Root / "control" / "blockReceipt" =>
        for {
          implicit0(log: Log[F]) ← logReq(req)
          receipt <- req.as[BlockReceipt]
          _ <- signals.enqueueReceipt(receipt)
          ok <- Ok()
        } yield ok

      case req @ (GET | POST) -> Root / "control" / "vmHash" =>
        for {
          implicit0(log: Log[F]) ← logReq(req)
          GetVmHash(height) <- req.as[GetVmHash]
          vmHash <- signals.getVmHash(height)
          ok <- Ok(vmHash.hash)
        } yield ok

      case req =>
        for {
          implicit0(log: Log[F]) ← logReq(req)
          _ ← Log[F].warn(s"RPC: unexpected request: ${req.method}")
        } yield Response.notFound
    }

    HttpRoutes
      .of[F](route)
      .orNotFound
  }

  /**
   * Create a resource with ControlServer, will close http server after use
   *
   * @param config Configuration, e.g., host and port to listen on
   * @tparam F Effect
   * @return
   */
  def make[F[_]: ConcurrentEffect: Timer: LogFactory: Log](
    config: Config
  ): Resource[F, ControlServer[F]] = {
    implicit val dsl: Http4sDsl[F] = new Http4sDsl[F] {}

    for {
      signals <- ControlSignals[F]()
      server ← BlazeServerBuilder[F]
        .bindHttp(config.port, config.host)
        .withHttpApp(controlService(signals))
        .resource
    } yield ControlServer(signals, server)
  }

}
