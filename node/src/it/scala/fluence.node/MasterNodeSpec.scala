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
import cats.syntax.apply._
import com.softwaremill.sttp.circe.asJson
import com.softwaremill.sttp.{SttpBackend, _}
import fluence.EitherTSttpBackend
import fluence.effects.docker.DockerIO
import fluence.effects.ethclient.EthClient
import fluence.node.config.{FluenceContractConfig, MasterConfig, NodeConfig}
import fluence.node.eth.FluenceContract
import fluence.node.status.{MasterStatus, StatusAggregator}
import fluence.node.workers.tendermint.ValidatorKey
import org.scalatest.{Timer ⇒ _, _}
import slogging.MessageFormatter.DefaultPrefixFormatter
import slogging.{LazyLogging, LogLevel, LoggerConfig, PrintLoggerFactory}
import eth.FluenceContractTestOps._

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

  private val nodeResource = for {
    sttpB ← sttpResource
    dockerIO ← DockerIO.make[IO]()

    masterConf = MasterConfig
      .load()
      .unsafeRunSync()
      .copy(rootPath = Files.createTempDirectory("masternodespec").toString)

    node ← {
      implicit val s = sttpB
      implicit val d = dockerIO

      val nodeConf = NodeConfig(
        masterConf.endpoints,
        ValidatorKey("", Base64.getEncoder.encodeToString(Array.fill(32)(5))),
        "vAs+M0nQVqntR6jjPqTsHpJ4bsswA3ohx05yorqveyc=",
        masterConf.worker,
        masterConf.tendermint
      )

      TestWorkersPool
        .make[IO]
        .flatMap(
          MasterNode
            .make[IO, IO.Par](masterConf, nodeConf, _)
        )
    }
    agg ← StatusAggregator.make(masterConf, node)
    _ ← MasterHttp.make("127.0.0.1", 5678, agg, node.pool)
  } yield (sttpB, node)

  def fiberResource[F[_]: Concurrent, A](f: F[A]): Resource[F, Unit] =
    Resource.make(Concurrent[F].start(f))(_.cancel).void

  val runningNode =
    nodeResource.flatMap {
      case res @ (_, n) ⇒ fiberResource(n.run).as(res)
    }

  "MasterNode" should {
    PrintLoggerFactory.formatter = new DefaultPrefixFormatter(false, false, true)
    LoggerConfig.factory = PrintLoggerFactory()
    LoggerConfig.level = LogLevel.DEBUG

    "provide status" in {
      runningNode.use {
        case (sttpB, node) ⇒
          implicit val s = sttpB
          logger.debug("Going to run the node")

          eventually[IO](
            getStatus(5678).map(_.ethState.lastBlock.get.number.get.toInt shouldBe 3).void,
            100.millis, 15.seconds
          )

      }.unsafeRunSync()

    }

    "run and delete apps properly" in {
      (runningNode, EthClient.make[IO]()).tupled.use {
        case ((sttpB, node), ethClient) ⇒
          implicit val s = sttpB

          val contractAddress = "0x9995882876ae612bfd829498ccd73dd962ec950a"
          val owner = "0x4180FC65D613bA7E1a385181a219F1DBfE7Bf11d"

          val contractConfig = FluenceContractConfig(owner, contractAddress)

          val contract = FluenceContract(ethClient, contractConfig)

          for {
            _ ← contract.addNode[IO](node.nodeConfig, 10, 10)
            _ ← contract.addApp[IO]("llamadb", clusterSize = 1)
            _ ← eventually[IO](node.pool.getAll.map(_.size shouldBe 1), 100.millis, 15.seconds)

            id0 ← node.pool.getAll.map(_.head.appId)

            _ ← contract.addApp[IO]("llamadb", clusterSize = 1)
            _ ← eventually[IO](node.pool.getAll.map(_.size shouldBe 2), 100.millis, 15.seconds)

            _ ← contract.deleteApp[IO](id0)
            _ ← eventually[IO](node.pool.getAll.map(_.size shouldBe 1), 100.millis, 15.seconds)

            id1 ← node.pool.getAll.map(_.head.appId)
            _ ← contract.addApp[IO]("llamadb", clusterSize = 1)
            _ ← contract.deleteApp[IO](id1)
            _ ← eventually[IO](node.pool.getAll.map(_.size shouldBe 1), 100.millis, 15.seconds)

            _ ← contract.addApp[IO]("llamadb", clusterSize = 1)
            _ ← contract.addApp[IO]("llamadb", clusterSize = 1)
            _ ← contract.addApp[IO]("llamadb", clusterSize = 1)
            _ ← contract.addApp[IO]("llamadb", clusterSize = 1)
            _ ← contract.addApp[IO]("llamadb", clusterSize = 1)
            _ ← contract.addApp[IO]("llamadb", clusterSize = 1)
            _ ← contract.addApp[IO]("llamadb", clusterSize = 1)

            _ ← eventually[IO](node.pool.getAll.map(_.size shouldBe 8), 100.millis, 15.seconds)

            id2 ← node.pool.getAll.map(_.last.appId)
            _ ← contract.deleteApp[IO](id2)

            _ ← eventually[IO](node.pool.getAll.map(_.size shouldBe 7), 100.millis, 15.seconds)

          } yield ()
      }.unsafeRunSync()
    }
  }
}
