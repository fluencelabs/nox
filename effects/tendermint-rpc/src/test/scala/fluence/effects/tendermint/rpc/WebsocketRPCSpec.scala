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
      blocks <- wrpc.subscribeNewBlock[IO]
    } yield (server, blocks)

    // TODO: I guess that now it's impossible to implement this test without sending & waiting for events.
//    "connect and disconnect" in {
//      val (events, requests) = resourcesF.use {
//        case (server, events) =>
//          for {
//            _ <- server.close()
//            result <- events.compile.toList
//            requests <- server.requests().compile.toList
//          } yield (result, requests)
//      }.unsafeRunSync()
//
//      events shouldBe empty
//      requests.collect { case Text(_, _) => }.size shouldBe 1
//    }

    "receive messages" in {
      val events = resourcesF.use {
        case (server, events) =>
          for {
            _ <- server.send(Text("first"))
            // TODO: for some reason, this message doesn't get sent o_O
            // TODO(cont): oh well, sometimes it does! There's a race O_O. Where? How? Why? WHO? The Doctor.
            _ <- server.send(Text("second"))
            result <- events.take(2).compile.toList
            _ <- server.close()
          } yield result
      }.unsafeRunSync()

      events.size shouldBe 2
      events.head shouldBe "first"
      events.tail.head shouldBe "second"
    }

    // TODO: How to implement this test? With fibers? Not sure about that.........................
//    "reconnect forever" in {
//      val badResourcesF = for {
//        server <- WebsocketServer.make[IO]
//        wrpc <- TendermintRpc.make[IO]("127.0.0.1", 12345)
//        blocks <- wrpc.subscribeNewBlock[IO]
//      } yield (server, blocks)
//
//      val result = badResourcesF.use(_ => IO.unit).attempt.unsafeRunSync()
//      println(s"Result: ${result.left.toOption.map(_.getMessage)}")
//      result.isRight shouldBe true
//    }
  }
}
