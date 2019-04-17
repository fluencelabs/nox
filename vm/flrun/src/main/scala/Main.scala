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

import cats.effect.{ExitCode, IO, IOApp}
import fluence.vm.WasmVm
import org.http4s.implicits._
import org.http4s.server.blaze.BlazeServerBuilder
import org.http4s.server.middleware.CORS
import org.http4s.{HttpApp, HttpRoutes, Response}
import slogging.LogLevel

object Main extends IOApp with slogging.LazyLogging {

  import Dsl._
  import Dsl.dsl._
  import Settings._
  import Utils._

  // apps/1/tx
  // apps/1/query?path=kALX917gZsqm%2F0&data=
  def routes(handler: TxProcessor[IO]): HttpRoutes[IO] = HttpRoutes.of[IO] {
    case req @ POST -> Root / "apps" / LongVar(appId) / "tx" ⇒
      logger.info(s"Tx request. appId: $appId")
      req.decode[String] { input ⇒
        val Array(path, tx) = input.split('\n')
        logger.info(s"Tx: '$tx'")
        handler.processTx(Tx(appId, path, tx)).handleErrorWith(e => BadRequest(e.getMessage))
      }

    case GET -> Root / "apps" / LongVar(appId) / "query" :? QueryPath(path) +& QueryData(data) ⇒
      logger.info(s"Query request. appId: $appId, path: $path, data: $data")
      handler.processQuery(Query(appId, path)).handleErrorWith(e => BadRequest(e.getMessage))
  }

  def app(handler: TxProcessor[IO]): HttpApp[IO] =
    CORS[IO, IO](routes(handler).orNotFound, corsConfig)

  override def run(args: List[String]): IO[ExitCode] = {
    configureLogging(LogLevel.DEBUG)
    for {
      files <- getWasmFiles()
      vmOrError <- WasmVm[IO](files, "fluence.vm.debugger").value
      vm <- IO.fromEither(vmOrError)
      processor <- TxProcessor[IO](vm)
      httpApp = app(processor)
      res = BlazeServerBuilder[IO].withBanner(Nil).bindHttp(Port, Host).withHttpApp(httpApp).resource
      _ <- res.use(_ => IO.never)
    } yield ExitCode.Success
  }
}
