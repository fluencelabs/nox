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
import fluence.effects.sttp.SttpEffect
import fluence.log.{Log, LogFactory}
import fluence.statemachine.abci.peers.{DropPeer, PeersControlBackend}
import fluence.statemachine.api.command.PeersControl
import fluence.statemachine.http.PeersControlHttp
import org.http4s.{HttpApp, Response}
import org.http4s.dsl.Http4sDsl
import org.http4s.server.Router
import org.http4s.server.blaze.BlazeServerBuilder
import org.scalatest.{Matchers, OptionValues, WordSpec}
import scodec.bits.ByteVector

import scala.concurrent.ExecutionContext.Implicits.global

class PeersControlClientSpec extends WordSpec with Matchers with OptionValues {
  "PeersControlClient" should {
    implicit val ioTimer: Timer[IO] = IO.timer(global)
    implicit val ioShift: ContextShift[IO] = IO.contextShift(global)

    implicit val logFactory = LogFactory.forPrintln[IO](Log.Error)
    implicit val log: Log[IO] = LogFactory[IO].init(getClass.getSimpleName).unsafeRunSync()

    val host = "localhost"
    val port: Short = 26652

    implicit val http4sDsl = Http4sDsl[IO]

    val server =
      for {
        backend <- Resource.liftF(PeersControlBackend[IO])
        _ ← BlazeServerBuilder[IO]
          .bindHttp(port, host)
          .withHttpApp(
            HttpApp(
              Router[IO](
                "/peers" -> PeersControlHttp.routes(backend)
              ).run(_).getOrElse(Response.notFound)
            )
          )
          .resource
      } yield backend

    val resources = for {
      backend <- server
      implicit0(s: SttpEffect[IO]) <- SttpEffect.plainResource[IO]
      client = StateMachineClient
        .readOnly[IO](host, port, EitherT.rightT(None))
        .extend[PeersControl[IO]](
          new PeersControlClient[IO](host, port)
        )
    } yield (backend, client)

    "send drop peer" in {
      resources.use {
        case (backend, client) =>
          for {
            key <- IO.pure(ByteVector.fill(32)(1))
            _ <- client.command[PeersControl[IO]].dropPeer(key).value.flatMap(IO.fromEither)
            received <- backend.dropPeers.use(IO.pure)
          } yield {
            received.size shouldBe 1
            received.head shouldBe DropPeer(key)
          }
      }.unsafeRunSync()
    }
  }
}
