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

package fluence.statemachine.control
import cats.effect.{ContextShift, IO, Resource, Timer}
import cats.implicits._
import com.softwaremill.sttp.asynchttpclient.cats.AsyncHttpClientCatsBackend
import fluence.log.{Log, LogFactory}
import fluence.statemachine.control.ControlServer.ControlServerConfig
import io.circe.Encoder
import org.scalatest.{EitherValues, Matchers, OptionValues, WordSpec}
import scodec.bits.ByteVector

import scala.concurrent.ExecutionContext.Implicits.global
import scala.util.Random

trait ControlServerOps extends EitherValues with OptionValues {
  import com.softwaremill.sttp.circe._
  import com.softwaremill.sttp.{SttpBackend, _}

  val config: ControlServerConfig

  def send[Req: Encoder](request: Req, path: String)(
    implicit b: SttpBackend[IO, Nothing]
  ): IO[Response[String]] = {
    sttp
      .body(request)
      .post(uri"http://${config.host}:${config.port}/control/$path")
      .send()
  }
}

class ControlServerSpec extends WordSpec with Matchers with ControlServerOps {
  val config = ControlServerConfig("localhost", 26662)

  "ControlServer" should {
    implicit val ioTimer: Timer[IO] = IO.timer(global)
    implicit val ioShift: ContextShift[IO] = IO.contextShift(global)
    implicit val logFactory = LogFactory.forPrintln[IO]()
    implicit val log: Log[IO] = LogFactory[IO].init(getClass.getSimpleName).unsafeRunSync()

    val server = ControlServer.make[IO](config)
    val sttp = Resource.make(IO(AsyncHttpClientCatsBackend[IO]()))(sttpBackend ⇒ IO(sttpBackend.close()))
    val resources = server.flatMap(srv => sttp.map(srv -> _))

    "respond with 404" in {
      resources.use {
        case (_, sttp) =>
          implicit val sttpBackend = sttp
          for {
            response <- send("", "wrongPath")
          } yield {
            response.code shouldBe 404
          }
      }.unsafeRunSync()
    }

    "receive DropPeer event" in {
      resources.use {
        case (server, sttp) =>
          implicit val sttpBackend = sttp
          for {
            cp <- IO.pure(DropPeer(ByteVector.fill(32)(1)))
            response <- send[DropPeer](cp, "dropPeer")
            received <- server.signals.dropPeers.use(IO.pure)
          } yield {
            response.code shouldBe 200
            response.body.right.value shouldBe ""

            received.size shouldBe 1
            received.head shouldBe cp
          }
      }.unsafeRunSync()
    }

    "receive several ChangePeer events" in {
      val count = 3
      resources.use {
        case (server, sttp) =>
          implicit val sttpBackend = sttp
          for {
            dps <- IO(Array.fill(count)(DropPeer(ByteVector.fill(32)(Random.nextInt()))).toList)
            _ <- dps.map(send(_, "dropPeer")).sequence
            received <- server.signals.dropPeers.use(IO.pure)
          } yield {

            received.size shouldBe 3
            received should contain theSameElementsAs dps
          }
      }.unsafeRunSync()
    }
  }
}
