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
import java.io.File
import java.net.InetAddress

import cats.effect._
import cats.syntax.applicativeError._
import cats.syntax.functor._
import cats.syntax.monadError._
import com.softwaremill.sttp.{SttpBackend, _}
import com.softwaremill.sttp.asynchttpclient.cats.AsyncHttpClientCatsBackend
import com.softwaremill.sttp.circe.asJson
import com.softwaremill.sttp.{SttpBackend, _}
import fluence.ethclient.EthClient
import fluence.ethclient.helpers.Web3jConverters
import fluence.ethclient.helpers.Web3jConverters.hexToBytes32
import fluence.node.docker.{DockerIO, DockerParams}
import fluence.node.eth.{FluenceContract, FluenceContractConfig}
import fluence.node.workers.WorkerRunning
import org.scalactic.source.Position
import org.scalatest.exceptions.{TestFailedDueToTimeoutException, TestFailedException}
import org.scalatest.time.Span
import org.scalatest.{Timer => _, _}
import slogging.MessageFormatter.DefaultPrefixFormatter
import slogging.{LazyLogging, LogLevel, LoggerConfig, PrintLoggerFactory}

import scala.concurrent.ExecutionContext.Implicits.global
import scala.concurrent.duration._
import scala.io.Source
import scala.language.higherKinds
import scala.sys.process.{Process, ProcessLogger}

import fluence.util.util._

/**
 * This test contains a single test method that checks:
 * - MasterNode connectivity with ganache-hosted Fluence smart contract
 * - MasterNode ability to load previous node clusters and subscribe to new clusters
 * - Successful cluster formation and starting blocks creation
 */
class MasterNodeIntegrationSpec
    extends WordSpec with LazyLogging with Matchers with BeforeAndAfterAll with OptionValues {

  implicit private val ioTimer: Timer[IO] = IO.timer(global)
  implicit private val ioShift: ContextShift[IO] = IO.contextShift(global)

  override protected def beforeAll(): Unit = {
    // TODO: It is needed to build vm-llamadb project explicitly for launch this test from Idea
    logger.info("bootstrapping npm")
    runCmd("npm install")

    logger.info("starting Ganache")
    runBackground("npm run ganache")

    logger.info("deploying contract to Ganache")
    runCmd("npm run migrate")
  }

  override protected def afterAll(): Unit = {
    logger.info("killing ganache")
    runCmd("pkill -f ganache")

    logger.info("stopping containers")
    // TODO: kill containers through Master's HTTP API
    runCmd("docker rm -f 01_worker_0 01_worker_1")
  }

  def getStatus(statusPort: Short)(implicit sttpBackend: SttpBackend[IO, Nothing]): IO[MasterStatus] = {
    import MasterStatus._
    for {
      resp <- sttp.response(asJson[MasterStatus]).get(uri"http://127.0.0.1:$statusPort/status").send()
    } yield resp.unsafeBody.right.get
  }

  //TODO: change check to Master's HTTP API
  def checkMasterRunning(containerId: String): IO[Unit] =
    IO {
      var line = ""
      scala.sys.process
        .Process(s"docker logs $containerId")
        .!!(ProcessLogger(_ => {}, o => line += o))
      line
    }.map(line => line should include("switching to the new clusters"))

  def getStatusPort(basePort: Short): Short = (basePort + 400).toShort

  def runTwoMasters(basePort: Short): Resource[IO, Seq[String]] =
    Resource.make {
      val master1Port: Short = basePort
      val master2Port: Short = (basePort + 1).toShort
      val status1Port: Short = getStatusPort(master1Port)
      val status2Port: Short = getStatusPort(master2Port)

      for {
        master1 <- runMaster(master1Port, master1Port, "master1", status1Port)
        master2 <- runMaster(master2Port, master2Port, "master2", status2Port)

        _ <- eventually[IO](checkMasterRunning(master1), maxWait = 15.seconds)
        _ <- eventually[IO](checkMasterRunning(master2), maxWait = 15.seconds)
      } yield Seq(master1, master2)
    } { masters =>
      val containers = masters.mkString(" ")
      IO { runCmd(s"docker rm -f $containers") }
    }

  def getRunningWorker(statusPort: Short)(implicit sttpBackend: SttpBackend[IO, Nothing]) =
    IO.suspend {
      getStatus(statusPort).map(_.workers.headOption.flatMap { w =>
        Option(w.asInstanceOf[WorkerRunning])
      })
    }

  def withEthSttpAndTwoMasters(basePort: Short): Resource[IO, (EthClient, SttpBackend[IO, Nothing])] =
    for {
      ethClient <- EthClient.makeHttpResource[IO]()
      sttp <- Resource.make(IO(AsyncHttpClientCatsBackend[IO]()))(sttpBackend ⇒ IO(sttpBackend.close()))
      _ <- runTwoMasters(basePort)
    } yield (ethClient, sttp)

  "MasterNodes" should {
    PrintLoggerFactory.formatter = new DefaultPrefixFormatter(false, false, false)
    LoggerConfig.factory = PrintLoggerFactory()
    LoggerConfig.level = LogLevel.INFO

    val contractAddress = "0x9995882876ae612bfd829498ccd73dd962ec950a"
    val owner = "0x4180FC65D613bA7E1a385181a219F1DBfE7Bf11d"

    logger.info(s"Docker host: '$dockerHost'")

    val contractConfig = FluenceContractConfig(owner, contractAddress)

    def runTwoWorkers(basePort: Short)(implicit ethClient: EthClient, sttp: SttpBackend[IO, Nothing]): IO[Unit] = {
      val contract = FluenceContract(ethClient, contractConfig)
      for {
        status1 <- getStatus(getStatusPort(basePort))
        status2 <- getStatus(getStatusPort((basePort + 1).toShort))

        _ <- contract.addNode[IO](status1.nodeConfig).attempt
        _ <- contract.addNode[IO](status2.nodeConfig).attempt
        _ <- contract.addApp[IO]("llamadb", clusterSize = 2)

        _ <- eventually[IO](
          for {
            c1s0 <- heightFromTendermintStatus(basePort)
            c1s1 <- heightFromTendermintStatus(basePort + 1)
            worker1 <- getRunningWorker(getStatusPort(basePort))
            worker2 <- getRunningWorker(getStatusPort((basePort + 1).toShort))
          } yield {
            c1s0 shouldBe Some(2)
            c1s1 shouldBe Some(2)
            worker1 shouldBe defined
            worker2 shouldBe defined
          },
          maxWait = 90.seconds
        )
      } yield {}
    }

    def deleteApp(basePort: Short): IO[Unit] = withEthSttpAndTwoMasters(basePort).use {
      case (ethClient, s) =>
        implicit val sttp = s
        val getStatus1 = getRunningWorker(getStatusPort(basePort))
        val getStatus2 = getRunningWorker(getStatusPort((basePort + 1).toShort))
        val contract = FluenceContract(ethClient, contractConfig)

        for {
          _ <- runTwoWorkers(basePort)(ethClient, s)

          // Check that we have 2 running workers from runTwoWorkers
          status1 <- getStatus1
          _ = status1 shouldBe defined
          status2 <- getStatus2
          _ = status2 shouldBe defined

          appIdHex = status1.value.info.appId
          appId = hexToBytes32(appIdHex)
          _ <- contract.deleteApp[IO](appId)
          _ <- eventually[IO](
            for {
              s1 <- getRunningWorker(getStatusPort(basePort))
              s2 <- getRunningWorker(getStatusPort((basePort + 1).toShort))
            } yield {
              // Check that workers run no more
              s1 should not be defined
              s2 should not be defined
            },
            10.seconds
          )
        } yield {}
    }

    "sync their workers with contract clusters" in {
      withEthSttpAndTwoMasters(25000).use {
        case (e, s) =>
          runTwoWorkers(25000)(e, s)
      }.unsafeRunSync()
    }

    "stop workers on AppDelete event" in {
      deleteApp(26000).unsafeRunSync()
    }
  }
}
