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
import org.http4s._
import org.http4s.circe._
import org.http4s.dsl.Http4sDsl
import org.http4s.implicits._
import org.http4s.server.Server
import org.http4s.server.blaze._

import scala.language.higherKinds

case class ControlServer[F[_]](signals: ControlSignals[F], http: Server[F])

object ControlServer {

  private def controlService[F[_]: Concurrent](
    signals: ControlSignals[F]
  )(implicit dsl: Http4sDsl[F]): Kleisli[F, Request[F], Response[F]] = {
    import dsl._

    implicit val decoder: EntityDecoder[F, ChangePeer] = jsonOf[F, ChangePeer]

    HttpRoutes
      .of[F] {
        case req @ POST -> Root / "control" / "changePeer" =>
          for {
            change <- req.as[ChangePeer]
            _ <- signals.changePeer(change)
            ok <- Ok()
          } yield ok
      }
      .orNotFound
  }

  def make[F[_]: ConcurrentEffect: Timer](config: ControlServerConfig): Resource[F, ControlServer[F]] = {
    implicit val dsl: Http4sDsl[F] = new Http4sDsl[F] {}

    for {
      signals <- Resource.liftF(ControlSignals[F])
      server ← BlazeServerBuilder[F]
        .bindHttp(config.port, config.host)
        .withHttpApp(controlService(signals))
        .resource
    } yield ControlServer(signals, server)
  }

}
