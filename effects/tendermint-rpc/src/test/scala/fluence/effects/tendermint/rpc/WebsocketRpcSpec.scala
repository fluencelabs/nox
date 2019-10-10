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

package fluence.effects.tendermint.rpc

import cats.effect._
import cats.syntax.flatMap._
import fluence.effects.sttp.{SttpEffect, SttpStreamEffect}
import fluence.log.{Log, LogFactory}
import org.http4s.websocket.WebSocketFrame.Text
import fluence.effects.testkit.Timed
import org.scalatest.{Matchers, WordSpec}

import scala.concurrent.ExecutionContext.Implicits.global
import scala.language.higherKinds

class WebsocketRpcSpec extends WordSpec with Matchers with Timed {
  implicit private val ioTimer: Timer[IO] = IO.timer(global)
  implicit private val ioShift: ContextShift[IO] = IO.contextShift(global)

  implicit private val log: Log[IO] = LogFactory.forPrintln[IO](Log.Off).init("WebsocketRpcSpec").unsafeRunSync()

  type STTP = SttpStreamEffect[IO]
  implicit private val sttpResource: STTP = SttpEffect.stream[IO]

  val Port: Int = 18080

  val TextOpCode = 1

  "WebsocketRpc" should {

    val resourcesF = for {
      server <- WebsocketServer.make[IO](Port)
      wrpc = new TestWRpc[IO]("127.0.0.1", Port)
      blocks = wrpc.subscribeNewBlock(None)
    } yield (server, blocks)

    def block(height: Long) = Text(TestData.block(height))

    "subscribe and receive messages" in {
      timed[IO](resourcesF.use {
        case (server, events) =>
          for {
            _ <- server.send(block(1))
            _ <- server.send(block(2))
            events <- events.take(2).compile.toList
            requests <- server.requests().take(1).compile.toList
            _ <- server.close()
          } yield {
            requests.count(_.opcode == TextOpCode) shouldBe 1
            events.size shouldBe 2
            events.head.header.height shouldBe 1L
            events.tail.head.header.height shouldBe 2L
          }
      }).unsafeRunSync()
    }

    "receive message after reconnect" in {
      val height = 1L

      timed[IO](resourcesF.use {
        case (server, events) =>
          for {
            _ <- events.compile.drain
            _ <- server.close()
            events <- WebsocketServer.make[IO](Port).use { newServer =>
              newServer.send(block(height)) >> events.take(1).compile.toList
            }
          } yield {
            events.size shouldBe 1
            events.head.header.height shouldBe height
          }
      }).unsafeRunSync()
    }

    // TODO: This test (if ran in a loop) always fails on 210-th iteration because subscribeNewBlock times out
    "ignore incorrect json messages" in {
      val incorrectMsg = "incorrect"
      val height = 1L

      timed[IO](resourcesF.use {
        case (server, events) =>
          for {
            _ <- server.send(Text(incorrectMsg))
            _ <- server.send(block(height))
            events <- events.take(1).compile.toList
            _ <- server.close()
          } yield {
            events.size shouldBe 1
            events.head.header.height shouldBe height
          }
      }).unsafeRunSync()
    }
  }
}
