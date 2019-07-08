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

import cats.effect.{IO, IOApp, _}
import cats.syntax.applicative._
import cats.syntax.flatMap._
import org.http4s.dsl.{Http4sDsl, _}
import org.http4s.implicits._
import org.http4s.server.blaze.BlazeServerBuilder
import org.http4s.{HttpApp, HttpRoutes, Response}

import scala.language.higherKinds

object SomeApp extends IOApp {

  override def run(args: List[String]): IO[ExitCode] = {

    implicit val dsl: Http4sDsl[IO] = new Http4sDsl[IO] {}

    import dsl._

    object KeyQ extends QueryParamDecoderMatcher[String]("key")
    object NeighborsQ extends OptionalQueryParamDecoderMatcher[Int]("cyka")

    object PositionXParam extends QueryParamDecoderMatcher[String]("x")
    object PositionYParam extends QueryParamDecoderMatcher[String]("y")
    object TimestampParam extends QueryParamDecoderMatcher[String]("timestamp")

    val routes = HttpRoutes.of[IO] {
      case req @ GET -> Root / "kad" / "lookup" :? KeyQ(key) +& NeighborsQ(n) ⇒
        Response[IO](Ok).withEntity("KAK DELA").pure[IO]

      case GET -> Root / "lines" :? PositionXParam(x) +& PositionYParam(y) +& TimestampParam(time) =>
        Response[IO](Ok).withEntity("LINES LINES LINES").pure[IO]
    }

    val app: HttpApp[IO] = routes.orNotFound
    BlazeServerBuilder[IO]
      .bindHttp(5678, "localhost")
      .withHttpApp(app)
      .resource
      .use(_ => IO.never)
      .map(_ => ExitCode.Success)
  }
}
