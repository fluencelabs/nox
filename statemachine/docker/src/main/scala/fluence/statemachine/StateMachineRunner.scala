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
import cats.effect.{Concurrent, ExitCode, IO, IOApp}
import cats.syntax.flatMap._
import fluence.log.{Log, LogFactory}
import fluence.statemachine.abci.AbciHandler
import fluence.statemachine.abci.peers.PeersControlBackend
import fluence.statemachine.api.StateMachine
import fluence.statemachine.api.command.{PeersControl, ReceiptBus}
import fluence.statemachine.http.StateMachineHttp
import org.http4s.{Request, Response}
import org.http4s.dsl.Http4sDsl
import org.http4s.server.Router
import org.http4s.server.blaze.BlazeServerBuilder
import shapeless._

import scala.language.higherKinds

/**
 * Main class for the State machine.
 *
 * Launches JVM ABCI Server by instantiating jTendermint's `TSocket` registering `ABCIHandler` as handler.
 * jTendermint implements RPC layer, provides dedicated threads (`Consensus`, `Mempool` and `Query` thread
 * according to Tendermint specification) and sends ABCI requests to `ABCIHandler`.
 */
object StateMachineRunner extends IOApp {

  override def run(args: List[String]): IO[ExitCode] =
    StateMachineConfig
      .load[IO]()
      .flatMap { config =>
        val logLevel = Log.level(config.logLevel).getOrElse(Log.Info)
        implicit val logFactory: LogFactory[IO] =
          LogFactory.forPrintln[IO](logLevel)
        for {
          implicit0(log: Log[IO]) ← logFactory.init("server")
          _ ← log.info("Building State Machine ABCI handler")
          moduleFiles ← config
            .collectModuleFiles[IO]
            .value
            .flatMap(e ⇒ IO.fromEither(e.left.map(err ⇒ new RuntimeException(s"Can't parse VM module files: $err"))))

          peersBackend ← PeersControlBackend[IO]
          machine ← EmbeddedStateMachine
            .init[IO](
              moduleFiles,
              config.blockUploadingEnabled
            )
            .map(_.extend[PeersControl[IO]](peersBackend))
            .value
            .flatMap(
              e ⇒ IO.fromEither(e.left.map(err ⇒ new RuntimeException(s"Cannot initiate EmbeddedStateMachine: $err")))
            )

          _ <- (
            for {
              _ ← AbciHandler.make(config.abciPort, machine, peersBackend)

              _ ← BlazeServerBuilder[IO]
                .bindHttp(config.http.port, config.http.host)
                .withHttpApp(routes(machine))
                .resource
            } yield IO.never
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

  private def routes[F[_]: Concurrent: LogFactory, C <: HList](
    machine: StateMachine.Aux[F, C]
  )(
    implicit hb: ops.hlist.Selector[C, ReceiptBus[F]],
    pc: ops.hlist.Selector[C, PeersControl[F]]
  ) = {
    implicit val dsl: Http4sDsl[F] = Http4sDsl[F]

    val rs =
      Router[F](
        StateMachineHttp.routes(machine): _*
      )

    Kleisli[F, Request[F], Response[F]](
      a =>
        rs.run(a)
          .getOrElse(
            Response.notFound
              .withEntity(s"Route for ${a.method} ${a.pathInfo} ${a.params.mkString("&")} not found")
          )
    )
  }

}
