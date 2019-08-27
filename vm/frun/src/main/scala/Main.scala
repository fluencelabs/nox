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
import cats.syntax.apply._
import fluence.log.{Log, LogFactory}
import fluence.vm.WasmVm
import fluence.vm.wasm.MemoryHasher
import org.http4s.implicits._
import org.http4s.server.blaze.BlazeServerBuilder
import org.http4s.server.middleware.CORS
import org.http4s.{HttpApp, HttpRoutes}

import scala.util.control.NoStackTrace

case class UnknownLanguage(language: String) extends NoStackTrace {
  override def getMessage: String = s"Unsupported language: $language. Supported languages are: rust, wasm"
}

object Main extends IOApp {

  import Dsl._
  import Dsl.dsl._
  import Settings._
  import Utils._

  private val logFactory = LogFactory.forPrintln[IO](Log.Debug)

  // apps/1/tx
  // apps/1/query?path=kALX917gZsqm%2F0&data=
  def routes(handler: TxProcessor[IO]): HttpRoutes[IO] = HttpRoutes.of[IO] {
    case req @ POST -> Root / "apps" / LongVar(appId) / "tx" ⇒
      logFactory.init("apps/tx" -> "", "appId" -> appId.toString).flatMap { implicit log: Log[IO] ⇒
        req.decode[String] { input ⇒
          val (path, tx) = {
            val (p, txn) = input.span(_ != '\n')
            (p, txn.tail)
          }
          log.scope("tx.head" -> path)(
            log =>
              log.info(s"Tx: '$tx'") *>
                handler
                  .processTx(Tx(appId, path, tx))(log)
                  .handleErrorWith(e => log.error(s"Error on processing tx $tx", e) *> BadRequest(e.getMessage))
          )
        }
      }

    case GET -> Root / "apps" / LongVar(appId) / "query" :? QueryPath(path) +& QueryData(data) ⇒
      logFactory.init("apps/query").flatMap { implicit log: Log[IO] ⇒
        log.info(s"Query request. appId: $appId, path: $path, data: $data") *>
          handler
            .processQuery(Query(appId, path))
            .handleErrorWith(e => log.error("Error on processing query", e) *> BadRequest(e.getMessage))
      }

    case req @ POST -> Root / "apps" / LongVar(appId) / "txWaitResponse" ⇒
      logFactory.init("apps/txWaitResponse").flatMap { implicit log: Log[IO] ⇒
        log.info(s"txWaitResponse request. appId: $appId") *>
          req.decode[String] { input ⇒
            val (path, tx) = input.splitAt(input.indexOf('\n'))
            log.info(s"txWaitResponse: '$tx'") *>
              handler
                .processTx(Tx(appId, path, tx))
                .handleErrorWith(e => log.error("Error on processing tx", e) *> BadRequest(e.getMessage))
          }
      }
  }

  def app(handler: TxProcessor[IO]): HttpApp[IO] =
    CORS[IO, IO](routes(handler).orNotFound, corsConfig)

  def getFilesToRun(language: Option[String]): IO[NonEmptyList[String]] =
    language.map(_.toLowerCase) match {
      case None | Some("wasm") | Some("webassembly") => getWasmFiles(WasmCodeDirectory)
      case Some("rust")                              => Rust.compile()
      case Some(lang)                                => IO.raiseError(UnknownLanguage(lang))
    }

  override def run(args: List[String]): IO[ExitCode] =
    for {
      implicit0(log: Log[IO]) ← logFactory.init("run")
      files <- getFilesToRun(args.headOption)
      vmOrError <- WasmVm[IO](files, MemoryHasher[IO], "fluence.vm.debugger").value
      vm <- IO.fromEither(vmOrError)
      processor <- TxProcessor[IO](vm)
      httpApp = app(processor)
      res = BlazeServerBuilder[IO].withBanner(Nil).bindHttp(Port, Host).withHttpApp(httpApp).resource
      _ <- res.use(_ => IO.never)
    } yield ExitCode.Success
}
