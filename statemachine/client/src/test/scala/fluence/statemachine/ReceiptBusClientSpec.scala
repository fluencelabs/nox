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

import cats.effect.{ContextShift, IO, Resource, Timer}
import fluence.effects.sttp.SttpEffect
import fluence.log.{Log, LogFactory}
import fluence.statemachine.api.command.ReceiptBus
import fluence.statemachine.api.data.BlockReceipt
import fluence.statemachine.client.StateMachineClient
import fluence.statemachine.receiptbus.ReceiptBusBackend
import fluence.statemachine.http.ReceiptBusHttp
import org.http4s.dsl.Http4sDsl
import org.http4s.server.Router
import org.http4s.server.blaze.BlazeServerBuilder
import org.http4s.{HttpApp, Response}
import org.scalatest.{Matchers, OptionValues, WordSpec}
import scodec.bits.ByteVector

import scala.concurrent.ExecutionContext.Implicits.global
import scala.concurrent.duration._

class ReceiptBusClientSpec extends WordSpec with Matchers with OptionValues {
  "ReceiptBusClient" should {
    implicit val ioTimer: Timer[IO] = IO.timer(global)
    implicit val ioShift: ContextShift[IO] = IO.contextShift(global)

    implicit val logFactory = LogFactory.forPrintln[IO]()
    implicit val log: Log[IO] = LogFactory[IO].init(getClass.getSimpleName).unsafeRunSync()

    val host = "localhost"
    val port: Short = 26652

    implicit val http4sDsl = Http4sDsl[IO]

    val backendR =
      for {
        backend <- Resource.liftF(ReceiptBusBackend[IO])
        _ ← BlazeServerBuilder[IO]
          .bindHttp(port, host)
          .withHttpApp(
            HttpApp(
              Router[IO](
                "/receipt-bus" -> ReceiptBusHttp.routes(backend)
              ).run(_).getOrElse(Response.notFound)
            )
          )
          .resource
      } yield backend

    val resources = for {
      signals <- backendR
      implicit0(s: SttpEffect[IO]) <- SttpEffect.plainResource[IO]
      client = StateMachineClient[IO](host, port)
    } yield (signals, client)

    "send blockReceipt" in {
      val receipt = BlockReceipt(1, ByteVector(1, 2, 3))
      resources.use {
        case (backend, client) =>
          for {
            _ <- client.command[ReceiptBus[IO]].sendBlockReceipt(receipt).value.flatMap(IO.fromEither)
            after <- IO.pure(backend.getReceipt(1).unsafeRunTimed(1.second))
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
        case (backend, client) =>
          for {
            _ <- backend.enqueueVmHash(height, vmHash)
            after <- client.command[ReceiptBus[IO]].getVmHash(height).value.flatMap(IO.fromEither)
          } yield {
            after shouldBe vmHash
          }
      }.unsafeRunSync()
    }
  }
}
