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
import cats.effect.concurrent.Ref
import cats.effect.{ExitCode, IO, IOApp}
import cats.syntax.flatMap._
import fluence.vm.WasmVm
import org.http4s.dsl.Http4sDsl
import org.http4s.implicits._
import org.http4s.server.blaze.BlazeServerBuilder
import org.http4s.server.middleware.{CORS, CORSConfig}
import org.http4s.{HttpApp, HttpRoutes, Response}
import scodec.bits.ByteVector

import scala.concurrent.duration._

class Main extends IOApp with slogging.LazyLogging {
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
  def routes(vm: WasmVm, map: Ref[IO, Map[String, String]]): HttpRoutes[IO] = HttpRoutes.of[IO] {
    case req@POST -> Root / LongVar(appId) / "tx" :? QueryId(id) ⇒
      logger.info(s"Tx request. appId: $appId")
      req.decode[String] { tx ⇒
        for {
          result <- IO.fromEither(vm.invoke(None, tx.getBytes()).value)
          encoded = ByteVector(result).toBase64
          _ <- id.fold(IO.unit)(idValue => map.update(_.updated(idValue, encoded)))
        } yield result
      }

    case GET -> Root / LongVar(appId) / "query" :? QueryPath(path) +& QueryData(data) +& QueryId(id) ⇒
      logger.info(s"Query request. appId: $appId, path: $path, data: $data")

      for {
        result <- id.fold(IO.pure(Option.empty[String]))(idValue => map.get.map(_.get(idValue)))
        response = result.map(Ok(_)).getOrElse[Response[IO]](Response(NotFound))
      } yield response
  }

  def app(vm: WasmVm, map: Ref[IO, Map[String, String]]): HttpApp[IO] = CORS[IO, IO](routes(vm, map).orNotFound, corsConfig)

  override def run(args: List[String]): IO[ExitCode] = {
    for {
      map <- Ref.of[IO, Map[String, String]](Map.empty[String, String])
      vmOrError <- WasmVm[IO](NonEmptyList.one(CodeDirectory), "fluence.vm.debugger").value
      vm <- IO.fromEither(vmOrError)
      httpApp = app(vm, map)
      res = BlazeServerBuilder[IO].bindHttp(Port, Host).withHttpApp(httpApp).resource
      _ <- res.use(_ => IO.pure(ExitCode.Success))
    } yield ExitCode.Success
  }
}
