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

package fluence.statemachine

import cats.data.Kleisli
import cats.effect.ExitCase.{Canceled, Completed, Error}
import cats.effect.concurrent.Deferred
import cats.effect.{Concurrent, ExitCode, IO, IOApp}
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.log.{Log, LogFactory, LogLevel}
import fluence.statemachine.abci.AbciHandler
import fluence.statemachine.api.data.StateMachineStatus
import fluence.statemachine.http.ControlServer
import org.http4s.{Request, Response, Status}
import org.http4s.dsl.Http4sDsl
import org.http4s.server.Router
import org.http4s.server.blaze.BlazeServerBuilder

import scala.language.higherKinds

/**
 * Main class for the State machine.
 *
 * Launches JVM ABCI Server by instantiating jTendermint's `TSocket` registering `ABCIHandler` as handler.
 * jTendermint implements RPC layer, provides dedicated threads (`Consensus`, `Mempool` and `Query` thread
 * according to Tendermint specification) and sends ABCI requests to `ABCIHandler`.
 */
object ServerRunner extends IOApp {

  override def run(args: List[String]): IO[ExitCode] =
    StateMachineConfig
      .load[IO]()
      .flatMap { config =>
        val logLevel = LogLevel.fromString(config.logLevel).getOrElse(LogLevel.INFO)
        implicit val logFactory: LogFactory[IO] =
          LogFactory.forPrintln[IO](logLevel)
        for {
          implicit0(log: Log[IO]) ← logFactory.init("server")
          _ ← log.info("Building State Machine ABCI handler")
          statusDef ← Deferred[IO, IO[StateMachineStatus]]

          moduleFiles ← config
            .collectModuleFiles[IO]
            .value
            .flatMap(e ⇒ IO.fromEither(e.left.map(err ⇒ new RuntimeException(s"Can't parse VM module files: $err"))))
          abciConfig = AbciHandler.Config(moduleFiles, config.abciPort, config.blockUploadingEnabled)

          _ <- (
            for {
              signals ← ControlSignals[IO]()

              _ ← BlazeServerBuilder[IO]
                .bindHttp(config.http.port, config.http.host)
                .withHttpApp(routes(signals, statusDef.get.flatten))
                .resource

              _ ← AbciHandler.make[IO](abciConfig, statusDef, signals)
            } yield signals.stop
          ).use(identity)
        } yield ExitCode.Success
      }
      .guaranteeCase {
        case Canceled =>
          LogFactory.forPrintln[IO]().init("server", "shutdown") >>= (_.error("StateMachine was canceled"))
        case Error(e) =>
          LogFactory.forPrintln[IO]().init("server", "shutdown") >>= (_.error("StateMachine stopped with error", e))
        case Completed =>
          LogFactory.forPrintln[IO]().init("server", "shutdown") >>= (_.info("StateMachine exited gracefully"))
      }

  private def routes[F[_]: Concurrent: LogFactory](signals: ControlSignals[F], status: F[StateMachineStatus]) = {
    implicit val dsl: Http4sDsl[F] = Http4sDsl[F]

    Kleisli[F, Request[F], Response[F]](
      a =>
        Router[F](
          "/control" -> ControlServer.routes(signals, status)
        ).run(a)
          .getOrElse(
            Response.notFound
              .withEntity(s"Route for ${a.method} ${a.pathInfo} ${a.params.mkString("&")} not found")
          )
    )
  }

}
