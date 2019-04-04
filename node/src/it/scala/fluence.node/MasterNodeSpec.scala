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

import java.nio.ByteBuffer
import java.nio.file.Files
import java.util.Base64

import cats.data.EitherT
import cats.effect._
import cats.syntax.functor._
import com.softwaremill.sttp.circe.asJson
import com.softwaremill.sttp.{SttpBackend, _}
import fluence.EitherTSttpBackend
import fluence.effects.docker.DockerIO
import fluence.node.config.{MasterConfig, NodeConfig}
import fluence.node.status.{MasterStatus, StatusAggregator}
import fluence.node.workers.tendermint.ValidatorKey
import org.scalatest.{Timer => _, _}
import slogging.MessageFormatter.DefaultPrefixFormatter
import slogging.{LazyLogging, LogLevel, LoggerConfig, PrintLoggerFactory}

import scala.concurrent.ExecutionContext.Implicits.global
import scala.concurrent.duration._
import scala.language.higherKinds

class MasterNodeSpec
    extends WordSpec with LazyLogging with Matchers with BeforeAndAfterAll with OptionValues with Integration
    with GanacheSetup {

  implicit private val ioTimer: Timer[IO] = IO.timer(global)
  implicit private val ioShift: ContextShift[IO] = IO.contextShift(global)

  type Sttp = SttpBackend[EitherT[IO, Throwable, ?], fs2.Stream[IO, ByteBuffer]]

  private val sttpResource: Resource[IO, SttpBackend[EitherT[IO, Throwable, ?], fs2.Stream[IO, ByteBuffer]]] =
    Resource.make(IO(EitherTSttpBackend[IO]()))(sttpBackend ⇒ IO(sttpBackend.close()))

  override protected def beforeAll(): Unit = {
    wireupContract()
  }

  override protected def afterAll(): Unit = {
    killGanache()
  }

  def getStatus(statusPort: Short)(implicit sttpBackend: Sttp): IO[MasterStatus] = {
    import MasterStatus._
    (for {
      resp <- sttp.response(asJson[MasterStatus]).get(uri"http://127.0.0.1:$statusPort/status").send()
    } yield resp.unsafeBody.right.get).value.map(_.right.get)
  }

  "MasterNode" should {
    PrintLoggerFactory.formatter = new DefaultPrefixFormatter(false, false, true)
    LoggerConfig.factory = PrintLoggerFactory()
    LoggerConfig.level = LogLevel.DEBUG

    "provide status" in {
      val masterConf =
        MasterConfig.load().unsafeRunSync().copy(rootPath = Files.createTempDirectory("masternodespec").toString)

      val nodeConf = NodeConfig(
        masterConf.endpoints,
        ValidatorKey("", Base64.getEncoder.encodeToString(Array.fill(32)(5))),
        "127.0.0.1",
        masterConf.worker,
        masterConf.tendermint
      )

      val resource = for {
        sttpB ← sttpResource
        dockerIO ← DockerIO.make[IO]()
        node ← {
          implicit val s = sttpB
          implicit val d = dockerIO
          MasterNode
            .make[IO, IO.Par](masterConf, nodeConf, Files.createTempDirectory("masternodespec"))
        }
        agg ← StatusAggregator.make(masterConf, node)
      _ ← MasterHttp.make("127.0.0.1", 5678, agg, node.pool)
      } yield (sttpB, node)

      resource.use {
        case (sttpB, node) ⇒
          implicit val s = sttpB
          logger.debug("Going to run the node")
          for {
            fiber ← Concurrent[IO].start(node.run)
            _ = logger.debug("Node is running")
            _ ← eventually[IO](getStatus(5678).map(_.ethState.lastBlock.get.number.get.toInt shouldBe 3).void, 1.second, 15.seconds)
          _ ← fiber.cancel
          } yield ()

      }.unsafeRunSync()

    }
  }
}
