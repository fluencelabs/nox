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

import cats.effect.{ContextShift, IO, Timer}
import fluence.effects.sttp.SttpEffect
import fluence.log.{Log, LogFactory}
import fluence.statemachine.api.signals.{BlockReceipt, DropPeer}
import fluence.statemachine.api.{StateHash, StateMachineStatus}
import fluence.statemachine.control.signals.ControlSignals
import fluence.statemachine.http.ControlServer
import org.http4s.{HttpApp, Response}
import org.http4s.dsl.Http4sDsl
import org.http4s.server.Router
import org.http4s.server.blaze.BlazeServerBuilder
import org.scalatest.{Matchers, OptionValues, WordSpec}
import scodec.bits.ByteVector

import scala.concurrent.ExecutionContext.Implicits.global
import scala.concurrent.duration._

class ControlRpcSpec extends WordSpec with Matchers with OptionValues {
  "ControlRpc" should {
    implicit val ioTimer: Timer[IO] = IO.timer(global)
    implicit val ioShift: ContextShift[IO] = IO.contextShift(global)

    implicit val logFactory = LogFactory.forPrintln[IO]()
    implicit val log: Log[IO] = LogFactory[IO].init(getClass.getSimpleName).unsafeRunSync()

    val host = "localhost"
    val port: Short = 26652

    val status = IO(StateMachineStatus(false, StateHash.empty))

    implicit val http4sDsl = Http4sDsl[IO]

    val serverSignalsR =
      for {
        signals ← ControlSignals[IO]()
        _ ← BlazeServerBuilder[IO]
          .bindHttp(port, host)
          .withHttpApp(
            HttpApp(
              Router[IO](
                "/control" -> ControlServer.routes(signals, status)
              ).run(_).getOrElse(Response.notFound)
            )
          )
          .resource
      } yield signals

    val resources = for {
      signals <- serverSignalsR
      implicit0(s: SttpEffect[IO]) <- SttpEffect.plainResource[IO]
      rpc = ControlRpc[IO](host, port)
    } yield (signals, rpc)

    "return OK on status" in {
      resources.use { case (_, rpc) => rpc.status.value }.unsafeRunSync() should be('right)
    }

    "send drop peer" in {
      resources.use {
        case (signals, rpc) =>
          for {
            key <- IO.pure(ByteVector.fill(32)(1))
            _ <- rpc.dropPeer(key).value.flatMap(IO.fromEither)
            received <- signals.dropPeers.use(IO.pure)
          } yield {
            received.size shouldBe 1
            received.head shouldBe DropPeer(key)
          }
      }.unsafeRunSync()
    }

    "send stop" in {
      resources.use {
        case (signals, rpc) =>
          for {
            before <- IO.pure(signals.stop.unsafeRunTimed(0.seconds))
            _ <- rpc.stop.value.flatMap(IO.fromEither)
            after <- IO.pure(signals.stop.unsafeRunTimed(0.seconds))
          } yield {
            before should not be defined
            after shouldBe defined
          }
      }.unsafeRunSync()
    }

    "send blockReceipt" in {
      val receipt = BlockReceipt(1, ByteVector(1, 2, 3))
      resources.use {
        case (signals, rpc) =>
          for {
            _ <- rpc.sendBlockReceipt(receipt).value.flatMap(IO.fromEither)
            after <- IO.pure(signals.getReceipt(1).unsafeRunTimed(1.second))
          } yield {
            after shouldBe defined
            after.value.height shouldBe receipt.height
            after.value.bytes shouldBe receipt.bytes
          }
      }.unsafeRunSync()
    }

    "get vmHash" in {
      val vmHash = ByteVector(1, 2, 3)
      val height = 123L
      resources.use {
        case (signals, rpc) =>
          for {
            _ <- signals.enqueueStateHash(height, vmHash)
            after <- IO.pure(rpc.getVmHash(height).value.flatMap(IO.fromEither).unsafeRunTimed(1.seconds))
          } yield {
            after shouldBe defined
            after.value shouldBe vmHash
          }
      }.unsafeRunSync()
    }
  }
}
