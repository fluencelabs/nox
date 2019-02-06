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

import java.nio.file.Files
import java.util.Base64

import cats.effect._
import cats.syntax.functor._
import com.softwaremill.sttp.asynchttpclient.cats.AsyncHttpClientCatsBackend
import com.softwaremill.sttp.circe.asJson
import com.softwaremill.sttp.{SttpBackend, _}
import fluence.node.config.{MasterConfig, NodeConfig}
import fluence.node.status.{MasterStatus, StatusAggregator}
import fluence.node.workers.tendermint.ValidatorKey
import org.scalatest.{Timer ⇒ _, _}
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

  private val sttpResource: Resource[IO, SttpBackend[IO, Nothing]] = Resource
    .make(IO(AsyncHttpClientCatsBackend[IO]()))(sttpBackend ⇒ IO(sttpBackend.close()))

  override protected def beforeAll(): Unit = {
    wireupContract()
  }

  override protected def afterAll(): Unit = {
    killGanache()
  }

  def getStatus(statusPort: Short)(implicit sttpBackend: SttpBackend[IO, Nothing]): IO[MasterStatus] = {
    import MasterStatus._
    for {
      resp <- sttp.response(asJson[MasterStatus]).get(uri"http://127.0.0.1:$statusPort/status").send()
    } yield {
      resp.unsafeBody.right.get
    }
  }

  "MasterNode" should {
    PrintLoggerFactory.formatter = new DefaultPrefixFormatter(false, false, false)
    LoggerConfig.factory = PrintLoggerFactory()
    LoggerConfig.level = LogLevel.DEBUG

    "provide status" in {
      val masterConf =
        MasterConfig.load().unsafeRunSync().copy(tendermintPath = Files.createTempDirectory("masternodespec").toString)

      val nodeConf = NodeConfig(
        masterConf.endpoints,
        ValidatorKey("", Base64.getEncoder.encodeToString(Array.fill(32)(5))),
        "127.0.0.1",
        masterConf.worker
      )

      val resource = for {
        sttpB ← sttpResource
        node ← {
          implicit val s = sttpB
          MasterNode
            .make[IO, IO.Par](masterConf, nodeConf, Files.createTempDirectory("masternodespec"))
        }
        _ ← StatusAggregator.makeHttpResource(masterConf, node)
      } yield (sttpB, node)

      resource.use {
        case (sttpB, node) ⇒
          implicit val s = sttpB
          logger.debug("Going to run the node")
          for {
            _ ← Concurrent[IO].start(node.run)
            _ = logger.debug("Node is running")
            _ ← eventually[IO](getStatus(5678).void, 1.second, 15.seconds)
          } yield ()

      }.unsafeRunSync()

    }
  }
}
