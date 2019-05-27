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
import io.circe.Json
import org.http4s.websocket.WebSocketFrame.Text
import org.scalatest.{Matchers, WordSpec}
import slogging.MessageFormatter.DefaultPrefixFormatter
import slogging.{LogLevel, LoggerConfig, PrintLoggerFactory}

import scala.concurrent.ExecutionContext.Implicits.global
import scala.language.higherKinds

class WebsocketRPCSpec extends WordSpec with Matchers with slogging.LazyLogging {
  implicit private val ioTimer: Timer[IO] = IO.timer(global)
  implicit private val ioShift: ContextShift[IO] = IO.contextShift(global)

  type STTP = SttpBackend[EitherT[IO, Throwable, ?], fs2.Stream[IO, ByteBuffer]]
  implicit private val sttpResource: STTP = EitherTSttpBackend[IO]()

  "WebsocketRPC" should {
    PrintLoggerFactory.formatter = new DefaultPrefixFormatter(false, false, true)
    LoggerConfig.factory = PrintLoggerFactory()
    LoggerConfig.level = LogLevel.ERROR

    val resourcesF = for {
      server <- WebsocketServer.make[IO]
      wrpc <- TendermintRpc.make[IO]("127.0.0.1", 8080)
      blocks = wrpc.subscribeNewBlock[IO]
    } yield (server, blocks)

    def text(text: String) = Text(
      s"""
         |{
         |  "jsonrpc": "2.0",
         |  "id": "1#event",
         |  "result": {
         |    "query": "tm.event = 'NewBlock'",
         |    "data": {
         |      "type": "tendermint/event/NewBlock",
         |      "value": "$text"
         |    }
         |  }
         |}
      """.stripMargin
    )

    def asString(json: Json) = json.as[String].right.get

    "subscribe and receive messages" in {
      val (events, requests) = resourcesF.use {
        case (server, events) =>
          for {
            _ <- server.send(text("first"))
            _ <- server.send(text("second"))
            result <- events.take(2).compile.toList
            _ <- server.close()
            requests <- server.requests().compile.toList
          } yield (result, requests)
      }.unsafeRunSync()

      requests.size shouldBe 1

      events.size shouldBe 2
      asString(events.head) shouldBe "first"
      asString(events.tail.head) shouldBe "second"
    }

    "parse block json correctly" in {
      val events = resourcesF.use {
        case (server, events) =>
          for {
            _ <- server.send(Text(TestData.block))
            result <- events.take(1).compile.toList
            _ <- server.close()
          } yield result
      }.unsafeRunSync()

      events.size shouldBe 1

      val block = Block(events.head)
      block.left.foreach(throw _)
      block.isRight shouldBe true
    }

    "receive message after reconnect" in {
      val msg = "yo"
      val events = resourcesF.use {
        case (server, events) =>
          for {
            _ <- server.close()
            _ <- WebsocketServer.make[IO].use { newServer =>
              newServer.send(text(msg))
            }
            result <- events.take(1).compile.toList
          } yield result
      }.unsafeRunSync()

      events.size shouldBe 1
      asString(events.head) shouldBe msg
    }

    "ignore incorrect json messages" in {
      val incorrectMsg = "incorrect"
      val msg = "correct"

      // To hide error about incorrect msg
      LoggerConfig.level = LogLevel.OFF

      val events = resourcesF.use {
        case (server, events) =>
          for {
            _ <- server.send(Text(incorrectMsg))
            _ <- server.send(text(msg))
            result <- events.take(1).compile.toList
            _ <- server.close()
          } yield result
      }.unsafeRunSync()

      LoggerConfig.level = LogLevel.ERROR

      events.size shouldBe 1
      asString(events.head) shouldBe msg
    }
  }
}
