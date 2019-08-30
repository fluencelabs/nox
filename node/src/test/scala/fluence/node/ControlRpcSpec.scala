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

import cats.effect.concurrent.Deferred
import cats.syntax.all._
import cats.effect.{Concurrent, ContextShift, IO, Timer}
import fluence.effects.sttp.SttpEffect
import fluence.effects.tendermint.block.history.Receipt
import fluence.log.{Log, LogFactory}
import fluence.node.workers.control.ControlRpc
import fluence.statemachine.control.{ControlServer, ControlStatus}
import fluence.statemachine.control.signals.DropPeer
import org.scalatest.{Matchers, OptionValues, WordSpec}
import scodec.bits.ByteVector

import scala.concurrent.ExecutionContext.Implicits.global
import scala.concurrent.duration._

class ControlRpcSpec extends WordSpec with Matchers with OptionValues {
  implicit val ioTimer: Timer[IO] = IO.timer(global)
  implicit val ioShift: ContextShift[IO] = IO.contextShift(global)

  implicit val logFactory = LogFactory.forPrintln[IO]()
  implicit val log: Log[IO] = LogFactory[IO].init(getClass.getSimpleName).unsafeRunSync()

  "Deferred" should {
    "not block" in {
      fs2.Stream
        .emits(Seq(1, 2, 3, 4))
        .evalMap { e =>
          for {
            _ <- log.info(s"$e start")
            d <- Deferred[IO, Int]
            _ <- Concurrent[IO].start(Timer[IO].sleep(10.seconds) >> log.info(s"$e slept") >> d.complete(e))
            ee <- d.get
            _ <- log.info(s"$e end $ee")
          } yield ee
        }
        .compile
        .drain
        .unsafeRunSync()
    }
  }

  "ControlRpc" should {
    val config = ControlServer.Config("localhost", 26652)
    val serverR = ControlServer.make[IO](config, IO(ControlStatus(false)))

    val resources = for {
      server <- serverR
      implicit0(s: SttpEffect[IO]) <- SttpEffect.plainResource[IO]
      rpc = ControlRpc[IO](config.host, config.port)
    } yield (server, rpc)

    "return OK on status" in {
      resources.use { case (_, rpc) => rpc.status }.unsafeRunSync()
    }

    "send drop peer" in {
      resources.use {
        case (server, rpc) =>
          for {
            key <- IO.pure(ByteVector.fill(32)(1))
            _ <- rpc.dropPeer(key).value.flatMap(IO.fromEither)
            received <- server.signals.dropPeers.use(IO.pure)
          } yield {
            received.size shouldBe 1
            received.head shouldBe DropPeer(key)
          }
      }.unsafeRunSync()
    }

    "send stop" in {
      resources.use {
        case (server, rpc) =>
          for {
            before <- IO.pure(server.signals.stop.unsafeRunTimed(0.seconds))
            _ <- rpc.stop.value.flatMap(IO.fromEither)
            after <- IO.pure(server.signals.stop.unsafeRunTimed(0.seconds))
          } yield {
            before should not be defined
            after shouldBe defined
          }
      }.unsafeRunSync()
    }

    "send blockReceipt" in {
      val receipt = Receipt(1, ByteVector(1, 2, 3))
      resources.use {
        case (server, rpc) =>
          for {
            _ <- rpc.sendBlockReceipt(receipt).value.flatMap(IO.fromEither)
            after <- IO.pure(server.signals.getReceipt(1).unsafeRunTimed(1.second))
          } yield {
            after shouldBe defined
            after.value.receipt shouldBe receipt
          }
      }.unsafeRunSync()
    }

    "get vmHash" in {
      val vmHash = ByteVector(1, 2, 3)
      val height = 123L
      resources.use {
        case (server, rpc) =>
          for {
            _ <- server.signals.enqueueVmHash(height, vmHash)
            after <- IO.pure(rpc.getVmHash(height).value.flatMap(IO.fromEither).unsafeRunTimed(1.seconds))
          } yield {
            after shouldBe defined
            after.value shouldBe vmHash
          }
      }.unsafeRunSync()
    }
  }
}
