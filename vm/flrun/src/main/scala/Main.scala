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

import cats.data.NonEmptyList
import cats.effect.{ExitCode, IO, IOApp}
import fluence.vm.WasmVm
import org.http4s.dsl.Http4sDsl
import org.http4s.implicits._
import org.http4s.server.blaze.BlazeServerBuilder
import org.http4s.server.middleware.{CORS, CORSConfig}
import org.http4s.{HttpApp, HttpRoutes}
import cats.data.Kleisli
import cats.effect._
import cats.syntax.flatMap._
import cats.syntax.functor._
import org.http4s._
import org.http4s.dsl.Http4sDsl
import org.http4s.implicits._
import org.http4s.server.Server
import org.http4s.server.blaze._

import scala.concurrent.duration._

class Main extends IOApp {
  val CodeDirectory = "/code"
  val Port = 30000
  val Host = "0.0.0.0"

  private val corsConfig = CORSConfig(
    anyOrigin = true,
    anyMethod = true,
    allowedMethods = Some(Set("GET", "POST")),
    allowCredentials = true,
    maxAge = 1.day.toSeconds
  )

//  val routes: HttpRoutes[IO] = Router[IO](
//    "/status" -> ???,
//    "/apps" -> ???
//  )

  implicit val dsl: Http4sDsl[IO] = new Http4sDsl[IO] {}
  import dsl._

  object QueryPath extends QueryParamDecoderMatcher[String]("path")
  object QueryData extends OptionalQueryParamDecoderMatcher[String]("data")
  object QueryId extends OptionalQueryParamDecoderMatcher[String]("id")

  // apps/1/tx
  // apps/1/query?path=kALX917gZsqm%2F0&data=
  def routes(vm: WasmVm): HttpRoutes[IO] = HttpRoutes.of[IO] {
    case req @ POST -> Root / LongVar(appId) / "tx" :? QueryId(id) ⇒
      req.decode[String] { tx ⇒

      }
  }

  def app(vm: WasmVm): HttpApp[IO] = CORS[IO, IO](routes(vm).orNotFound, corsConfig)

  override def run(args: List[String]): IO[ExitCode] = {
    for {
      vmOrError <- WasmVm[IO](NonEmptyList.one(CodeDirectory), "fluence.vm.debugger").value
      vm = IO.fromEither(vmOrError)
      httpApp = app(vm)
      res = BlazeServerBuilder[IO].bindHttp(Port, Host).withHttpApp(httpApp).resource
      _ <- res.use(_ => IO.pure(ExitCode.Success))
    } yield ExitCode.Success
  }
}
