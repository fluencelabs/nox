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

import cats.effect._
import cats.syntax.apply._
import com.softwaremill.sttp.asynchttpclient.fs2.AsyncHttpClientFs2Backend
import com.softwaremill.sttp.circe.asJson
import com.softwaremill.sttp.{SttpBackend, _}
import fluence.effects.ethclient.EthClient
import fluence.log.{Log, LogFactory}
import fluence.node.config.FluenceContractConfig
import fluence.node.eth.FluenceContractTestOps._
import fluence.node.eth.{FluenceContract, NodeEthState}
import fluence.node.status.MasterStatus
import org.scalatest.{Timer ⇒ _, _}
import eth.FluenceContractTestOps._
import fluence.effects.ethclient.helpers.Web3jConverters
import fluence.log.{Log, LogFactory}
import fluence.node.config.FluenceContractConfig
import fluence.effects.testkit.Timed
import fluence.node.workers.WorkerDocker
import fluence.worker.WorkerStatus

import scala.concurrent.ExecutionContext.Implicits.global
import scala.concurrent.duration._
import scala.language.higherKinds
import scala.sys.process._

/**
 * This test contains a single test method that checks:
 * - MasterNode connectivity with ganache-hosted Fluence smart contract
 * - MasterNode ability to load previous node clusters and subscribe to new clusters
 * - Successful cluster formation and starting blocks creation
 */
class MasterNodeIntegrationSpec
    extends WordSpec with Matchers with BeforeAndAfterAll with OptionValues with Timed with GetWorkerStatus
    with GanacheSetup with DockerSetup {

  type Sttp = SttpBackend[IO, fs2.Stream[IO, ByteBuffer]]

  implicit private val ioTimer: Timer[IO] = IO.timer(global)
  implicit private val ioShift: ContextShift[IO] = IO.contextShift(global)

  implicit private val log: Log[IO] =
    LogFactory.forPrintln[IO](Log.Error).init("MasterNodeIntegrationSpec").unsafeRunSync()

  private val sttpResource: Resource[IO, SttpBackend[IO, fs2.Stream[IO, ByteBuffer]]] =
    Resource.make(IO(AsyncHttpClientFs2Backend[IO]()))(sttpBackend ⇒ IO(sttpBackend.close()))

  // TODO integrate with CLI, get app id from tx
  @volatile private var lastAppId = 1L

  override protected def beforeAll(): Unit = {
    wireupContract()
  }

  override protected def afterAll(): Unit = {
    killGanache()
  }

  def getStatus(statusPort: Short)(implicit sttpBackend: Sttp): IO[Either[RuntimeException, MasterStatus]] = {
    import MasterStatus._
    for {
      resp <- sttp.response(asJson[MasterStatus]).get(uri"http://127.0.0.1:$statusPort/status").send()
    } yield {
      resp.body.flatMap(_.left.map(e => s"${e.message} ${e.original} ${e.error}")).left.map(new RuntimeException(_))
    }
  }

  def getEthState(statusPort: Short)(implicit sttpBackend: Sttp): IO[NodeEthState] = {
    import NodeEthState._
    for {
      resp <- sttp.response(asJson[NodeEthState]).get(uri"http://127.0.0.1:$statusPort/status/eth").send()
    } yield {
      resp.unsafeBody.right.get
    }
  }

  def checkMasterRunning(statusPort: Short)(implicit sttpBackend: Sttp): IO[Unit] =
    getEthState(statusPort).map(_.contractAppsLoaded shouldBe true)

  def runTwoMasters(
    master1Port: Short,
    master2Port: Short
  )(implicit sttpBackend: Sttp): Resource[IO, (String, String)] = {
    for {
      master1 <- runMaster(master1Port, "master1", n = 1)
      master2 <- runMaster(master2Port, "master2", n = 2)

      _ <- Resource liftF eventually[IO](
        checkMasterRunning(master1Port) *> checkMasterRunning(master2Port),
        maxWait = 45.seconds
      ) // TODO: 45 seconds is a bit too much for startup; investigate and reduce timeout

    } yield (master1, master2)
  }

  def getRunningWorker(statusPort: Short)(implicit sttpBackend: Sttp): IO[Option[WorkerStatus]] =
    IO.suspend {
      getStatus(statusPort).map {
        case Right(st) =>
          st.workers.headOption.flatMap {
            case (_, w: WorkerStatus) if w.isOperating =>
              Some(w)
            case _ ⇒
              log.debug("Trying to get WorkerRunning, but it is not healthy in status: " + st).unsafeRunSync()
              None
          }
        case Left(e) =>
          log.error(s"Error on getting worker status at $statusPort: $e").unsafeRunSync()
          None
      }
    }

  def withEthSttpAndTwoMasters(
    master1Port: Short,
    master2Port: Short
  ): Resource[IO, (EthClient, Sttp, String, String)] =
    for {
      ethClient <- EthClient.make[IO]()
      implicit0(sttp: Sttp) <- sttpResource
      (master1ContainerId, master2ContainerId) <- runTwoMasters(master1Port, master2Port)
    } yield (ethClient, sttp, master1ContainerId, master2ContainerId)

  def tendermintNodeId(masterContainerId: String) = {
    IO(
      s"docker run --user 0 --rm --volumes-from $masterContainerId -e TMHOME=/master/tendermint tendermint/tendermint show_node_id".!!
    )
  }

  "MasterNodes" should {
    val contractAddress = "0x9995882876ae612bfd829498ccd73dd962ec950a"
    val owner = "0x4180FC65D613bA7E1a385181a219F1DBfE7Bf11d"

    log.info(s"Docker host: '$dockerHost'").unsafeRunSync()

    val contractConfig = FluenceContractConfig(owner, contractAddress)

    def runTwoWorkers(
      master1Port: Short,
      master2Port: Short,
      master1ContainerId: String,
      master2ContainerId: String
    )(implicit ethClient: EthClient, sttp: Sttp): IO[Unit] = {

      val contract = FluenceContract(ethClient, contractConfig)
      for {
        status1 <- getStatus(master1Port).map(_.toTry.get)
        status2 <- getStatus(master2Port).map(_.toTry.get)
        validatorKey1 <- getEthState(master1Port).map(_.validatorKey.toBase64)
        validatorKey2 <- getEthState(master2Port).map(_.validatorKey.toBase64)
        nodeId1 ← tendermintNodeId(master1ContainerId)
        nodeId2 ← tendermintNodeId(master2ContainerId)

        _ <- contract.addNode[IO](validatorKey1, nodeId1, status1.ip, master1Port, 1).attempt
        _ <- contract.addNode[IO](validatorKey2, nodeId2, status2.ip, master2Port, 1).attempt
        blockNumber <- contract.addApp[IO]("llamadb", clusterSize = 2)

        _ ← log.info("Added App at block: " + blockNumber + ", now going to wait for two workers")

        _ <- eventually[IO](
          for {
            status1 <- getWorkerStatus("localhost", master1Port, lastAppId)
            _ ← log.info(s"worker1 status: $status1")
            status2 <- getWorkerStatus("localhost", master2Port, lastAppId)
            _ ← log.info(s"worker2 status: $status2")
          } yield {
            status1.isOperating shouldBe true
            status2.isOperating shouldBe true
          },
          maxWait = 90.seconds
        )

        _ = lastAppId += 1

        _ ← log.info("Height equals two for workers, going to get WorkerRunning from Master status")

        _ <- eventually[IO](
          for {
            worker1 <- getRunningWorker(master1Port)
            worker2 <- getRunningWorker(master2Port)
          } yield {
            worker1 shouldBe defined
            worker2 shouldBe defined
          },
          maxWait = 90.seconds
        )
      } yield ()
    }

    def deleteApp(master1Port: Short, master2Port: Short): IO[Unit] =
      withEthSttpAndTwoMasters(master1Port, master2Port).use {
        case (ethClient, s, master1ContainerId, master2ContainerId) =>
          log.debug("Prepared two masters for Delete App test").unsafeRunSync()
          implicit val sttp = s
          val getStatus1 = getRunningWorker(master1Port)
          val getStatus2 = getRunningWorker(master2Port)
          val contract = FluenceContract(ethClient, contractConfig)

          for {
            _ <- runTwoWorkers(master1Port, master2Port, master1ContainerId, master2ContainerId)(ethClient, s)

            _ ← log.debug("Two workers should be running")

            // Check that we have 2 running workers from runTwoWorkers
            status1 <- getStatus1
            _ = status1 shouldBe defined
            _ ← log.debug("Worker1 running: " + status1)

            status2 <- getStatus2
            _ = status2 shouldBe defined
            _ ← log.debug("Worker2 running: " + status2)

            appId = 1L
            _ <- contract.deleteApp[IO](appId)
            _ ← log.debug("App deleted from contract")

            _ <- eventually[IO](
              for {
                s1 <- getStatus1
                s2 <- getStatus2
              } yield {
                // Check that workers run no more
                s1 should not be defined
                s2 should not be defined
              },
              maxWait = 30.seconds
            )
          } yield ()
      }

    "sync their workers with contract clusters" in {
      val master1Port: Short = 20000
      val master2Port: Short = 20001

      withEthSttpAndTwoMasters(master1Port, master2Port).use {
        case (e, s, m1Id, m2Id) =>
          runTwoWorkers(master1Port, master2Port, m1Id, m2Id)(e, s).flatMap(_ ⇒ IO("docker ps".!!))
      }.flatMap { psOutput ⇒
        println("CONTAINERS: " + psOutput)
        psOutput should include("worker")
        // Check that once masters are stopped, workers do not exist
        eventually(IO("docker ps -a".!! should not include "worker"))

      }.unsafeRunSync()
    }

    "stop workers on AppDelete event" in {
      deleteApp(21000, 21001).unsafeRunSync()
    }
  }
}
