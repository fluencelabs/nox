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

import java.nio.ByteBuffer

import cats.data.EitherT
import cats.effect._
import cats.syntax.flatMap._
import com.softwaremill.sttp.SttpBackend
import fluence.EitherTSttpBackend
import fluence.effects.tendermint.block.data.Block
import fluence.log.{Log, LogFactory}
import io.circe.Json
import org.http4s.websocket.WebSocketFrame.Text
import org.scalatest.{Matchers, WordSpec}

import scala.concurrent.ExecutionContext.Implicits.global
import scala.language.higherKinds

class WebsocketRpcSpec extends WordSpec with Matchers {
  implicit private val ioTimer: Timer[IO] = IO.timer(global)
  implicit private val ioShift: ContextShift[IO] = IO.contextShift(global)

  implicit private val log: Log[IO] = LogFactory.forPrintln[IO](Log.Error).init("WebsocketRpcSpec").unsafeRunSync()

  type STTP = SttpBackend[EitherT[IO, Throwable, ?], fs2.Stream[IO, ByteBuffer]]
  implicit private val sttpResource: STTP = EitherTSttpBackend[IO]()

  val Port: Int = 18080

  "WebsocketRpc" should {

    val resourcesF = for {
      server <- WebsocketServer.make[IO](Port)
      wrpc = new TestWRpc[IO]("127.0.0.1", Port)
      blocks = wrpc.subscribeNewBlock(0)
    } yield (server, blocks)

    def block(height: Long) = Text(TestData.block(height))

    "subscribe and receive messages" in {
      val (events, requests) = resourcesF.use {
        case (server, events) =>
          for {
            _ <- server.send(block(1))
            _ <- server.send(block(2))
            result <- events.take(2).compile.toList
            _ <- server.close()
            requests <- server.requests().compile.toList
          } yield (result, requests)
      }.unsafeRunSync()

      requests.size shouldBe 1

      events.size shouldBe 2
      events.head.header.height shouldBe 1L
      events.tail.head.header.height shouldBe 2L
    }

    "receive message after reconnect" in {
      val height = 1L
      val events = resourcesF.use {
        case (server, events) =>
          for {
            _ <- server.close()
            result <- WebsocketServer.make[IO](Port).use { newServer =>
              newServer.send(block(height)) >> events.take(1).compile.toList
            }
          } yield result
      }.unsafeRunSync()

      events.size shouldBe 1
      events.head.header.height shouldBe height
    }

    "ignore incorrect json messages" in {
      val incorrectMsg = "incorrect"
      val height = 1L

      val events = resourcesF.use {
        case (server, events) =>
          for {
            _ <- server.send(Text(incorrectMsg))
            _ <- server.send(block(height))
            result <- events.take(1).compile.toList
            _ <- server.close()
          } yield result
      }.unsafeRunSync()

      events.size shouldBe 1
      events.head.header.height shouldBe height
    }
  }
}
