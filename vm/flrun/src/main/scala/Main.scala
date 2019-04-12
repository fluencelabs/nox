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

import cats.effect.concurrent.Ref
import cats.effect.{ExitCode, IO, IOApp}
import cats.syntax.list._
import fluence.statemachine.config.StateMachineConfig
import fluence.statemachine.error.VmModuleLocationError
import fluence.vm.WasmVm
import org.http4s.dsl.Http4sDsl
import org.http4s.implicits._
import org.http4s.server.blaze.BlazeServerBuilder
import org.http4s.server.middleware.{CORS, CORSConfig}
import org.http4s.{HttpApp, HttpRoutes}
import scodec.bits.ByteVector
import slogging.MessageFormatter.DefaultPrefixFormatter
import slogging.{LogLevel, LoggerConfig, PrintLoggerFactory}

import scala.concurrent.duration._

object Main extends IOApp with slogging.LazyLogging {

  val CodeDirectory =
    "/Users/folex/Development/fluencelabs/fluence-main/vm/src/it/resources/test-cases/llamadb/target/wasm32-unknown-unknown/release/"
  //  val CodeDirectory = "Code"
  val Port = 30000
  val Host = "0.0.0.0"

  private val corsConfig = CORSConfig(
    anyOrigin = true,
    anyMethod = true,
    allowedMethods = Some(Set("GET", "POST")),
    allowCredentials = true,
    maxAge = 1.day.toSeconds
  )

  implicit val dsl: Http4sDsl[IO] = new Http4sDsl[IO] {}

  import dsl._

  object QueryPath extends QueryParamDecoderMatcher[String]("path")

  object QueryData extends OptionalQueryParamDecoderMatcher[String]("data")

  object QueryId extends QueryParamDecoderMatcher[String]("id")

  // apps/1/tx
  // apps/1/query?path=kALX917gZsqm%2F0&data=
  def routes(vm: WasmVm, map: Ref[IO, Map[String, String]]): HttpRoutes[IO] = HttpRoutes.of[IO] {
    case req @ POST -> Root / "apps" / LongVar(appId) / "tx" ⇒
      logger.info(s"Tx request. appId: $appId")
      req.decode[String] { input ⇒
        logger.info(s"Tx: '$input'")
        val Array(sessionCounter, tx) = input.split('\n')
        for {
          result <- vm.invoke[IO](None, tx.getBytes()).value.flatMap(IO.fromEither)
          encoded = ByteVector(result).toBase64
          _ <- map.update(_.updated(sessionCounter, encoded))
          json = s"""
                    | {
                    |  "jsonrpc": "2.0",
                    |  "id": "dontcare",
                    |  "result": {
                    |    "code": 0,
                    |    "data": "$encoded",
                    |    "hash": "no hash"
                    |  }
                    | }
            """.stripMargin
          response <- Ok(json)
        } yield response
      }

    case GET -> Root / "apps" / LongVar(appId) / "query" :? QueryPath(path) +& QueryData(data) ⇒
      logger.info(s"Query request. appId: $appId, path: $path, data: $data")

      for {
        result <- map.get.map(_.get(path)).map(_.getOrElse("not found"))
        json = s"""
                  |{
                  |  "jsonrpc": "2.0",
                  |  "id": "dontcare",
                  |  "result": {
                  |    "response": {
                  |      "info": "Responded for path $path",
                  |      "value": "$result"
                  |    }
                  |  }
                  |}
           """.stripMargin
        response <- Ok(json)
      } yield response
  }

  def app(vm: WasmVm, map: Ref[IO, Map[String, String]]): HttpApp[IO] =
    CORS[IO, IO](routes(vm, map).orNotFound, corsConfig)

  def getWasmFiles() =
    StateMachineConfig
      .listWasmFiles(CodeDirectory)
      .map(
        _.toNel.toRight(
          new RuntimeException(
            VmModuleLocationError("Provided directories don't contain any wasm or wast files").toString
          )
        )
      )
      .flatMap(IO.fromEither)

  def configureLogging(level: LogLevel): Unit = {
    PrintLoggerFactory.formatter =
      new DefaultPrefixFormatter(printLevel = true, printName = true, printTimestamp = true)
    LoggerConfig.factory = PrintLoggerFactory()
    LoggerConfig.level = level
  }

  override def run(args: List[String]): IO[ExitCode] = {
    configureLogging(LogLevel.DEBUG)
    for {
      files <- getWasmFiles()
      vmOrError <- WasmVm[IO](files, "fluence.vm.debugger").value
      vm <- IO.fromEither(vmOrError)
      map <- Ref.of[IO, Map[String, String]](Map.empty[String, String])
      httpApp = app(vm, map)
      res = BlazeServerBuilder[IO].withBanner(Nil).bindHttp(Port, Host).withHttpApp(httpApp).resource
      _ <- res.use(_ => IO.never)
    } yield ExitCode.Success
  }
}
