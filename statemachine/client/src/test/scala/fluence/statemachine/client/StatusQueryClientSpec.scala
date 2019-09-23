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

package fluence.statemachine.client

import cats.data.EitherT
import cats.effect.{ContextShift, IO, Resource, Timer}
import fluence.effects.EffectError
import fluence.effects.sttp.SttpEffect
import fluence.log.{Log, LogFactory}
import fluence.statemachine.api.StateMachine
import fluence.statemachine.api.command.PeersControl
import fluence.statemachine.api.data.{StateHash, StateMachineStatus}
import fluence.statemachine.api.query.{QueryCode, QueryResponse}
import fluence.statemachine.http.StateMachineHttp
import org.http4s.{HttpApp, Response}
import org.http4s.dsl.Http4sDsl
import org.http4s.server.blaze.BlazeServerBuilder
import org.scalatest.{Matchers, OptionValues, WordSpec}

import scala.concurrent.ExecutionContext.Implicits.global

class StatusQueryClientSpec extends WordSpec with Matchers with OptionValues {
  "StateMachineClient" should {
    implicit val ioTimer: Timer[IO] = IO.timer(global)
    implicit val ioShift: ContextShift[IO] = IO.contextShift(global)

    implicit val logFactory = LogFactory.forPrintln[IO]()
    implicit val log: Log[IO] = LogFactory[IO].init(getClass.getSimpleName).unsafeRunSync()

    val host = "localhost"
    val port: Short = 26652

    implicit val http4sDsl = Http4sDsl[IO]

    val backendR =
      for {
        backend <- Resource.pure[IO, StateMachine[IO]](new StateMachine.ReadOnly[IO] {
          override def query(path: String)(implicit log: Log[IO]): EitherT[IO, EffectError, QueryResponse] =
            EitherT.liftF(IO.pure(QueryResponse(1, Array.emptyByteArray, QueryCode.Ok, "info")))

          override def status()(implicit log: Log[IO]): EitherT[IO, EffectError, StateMachineStatus] =
            EitherT.liftF(IO.pure(StateMachineStatus(expectsEth = false, StateHash.empty)))
        })
        _ ← BlazeServerBuilder[IO]
          .bindHttp(port, host)
          .withHttpApp(
            HttpApp(
              StateMachineHttp.readRoutes(backend).run(_).getOrElse(Response.notFound)
            )
          )
          .resource
      } yield backend

    val resources = for {
      backend <- backendR
      implicit0(s: SttpEffect[IO]) <- SttpEffect.plainResource[IO]
      client = StateMachineClient
        .readOnly[IO](host, port)
        .extend[PeersControl[IO]](
          new PeersControlClient[IO](host, port)
        )
    } yield (backend, client)

    "return OK on status" in {
      resources.use { case (_, rpc) => rpc.status.value }.unsafeRunSync() should be('right)
    }

    "return OK query" in {
      resources.use { case (_, client) => client.query("").value }.unsafeRunSync() should be('right)
    }
  }
}
