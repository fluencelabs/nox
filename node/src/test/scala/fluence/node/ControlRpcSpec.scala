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

package fluence.node

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

import cats.effect.{ContextShift, IO, Resource, Timer}
import com.softwaremill.sttp.asynchttpclient.cats.AsyncHttpClientCatsBackend
import fluence.node.workers.control.ControlRpc
import fluence.statemachine.control.{ControlServer, DropPeer}
import fluence.statemachine.control.ControlServer.ControlServerConfig
import org.scalatest.{Matchers, WordSpec}
import scodec.bits.ByteVector
import scala.concurrent.duration._

import scala.concurrent.ExecutionContext.Implicits.global

class ControlRpcSpec extends WordSpec with Matchers {
  "ControlRpc" should {
    implicit val ioTimer: Timer[IO] = IO.timer(global)
    implicit val ioShift: ContextShift[IO] = IO.contextShift(global)

    val config = ControlServerConfig("localhost", 26662)
    val server = ControlServer.make[IO](config)

    val sttp = Resource.make(IO(AsyncHttpClientCatsBackend[IO]()))(sttpBackend ⇒ IO(sttpBackend.close()))
    val resources = for {
      server <- server
      s <- sttp
      rpc = {
        implicit val b = s
        ControlRpc[IO](config.host, config.port)
      }
    } yield (server, rpc)

    "return OK on status" in {
      resources.use { case (_, rpc) => rpc.status() }.unsafeRunSync()
    }

    "send drop peer" in {
      resources.use {
        case (server, rpc) =>
          for {
            key <- IO.pure(ByteVector.fill(32)(1))
            _ <- rpc.dropPeer(key)
            received <- server.signals.dropPeers.use(IO.pure)
          } yield {
            received.length shouldBe 1
            received.head shouldBe DropPeer(key)
          }
      }.unsafeRunSync()
    }

    "send stop" in {
      resources.use {
        case (server, rpc) =>
          for {
            before <- IO.pure(server.signals.stop.unsafeRunTimed(0.seconds))
            _ <- rpc.stop()
            after <- IO.pure(server.signals.stop.unsafeRunTimed(0.seconds))
          } yield {
            before should not be defined
            after shouldBe defined
          }
      }.unsafeRunSync()
    }
  }
}
