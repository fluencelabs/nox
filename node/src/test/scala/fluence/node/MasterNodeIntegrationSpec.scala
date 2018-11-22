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
import java.nio.file.Paths

import cats.effect.{ContextShift, IO, Resource, Timer}
import com.softwaremill.sttp.SttpBackend
import com.softwaremill.sttp.asynchttpclient.cats.AsyncHttpClientCatsBackend
import fluence.ethclient.EthClient
import fluence.node.eth.{DeployerContract, DeployerContractConfig}
import fluence.node.solvers.SolversPool
import fluence.node.tendermint.KeysPath
import org.scalatest.{BeforeAndAfterAll, FlatSpec, Matchers}
import slogging.MessageFormatter.DefaultPrefixFormatter
import slogging.{LazyLogging, LogLevel, LoggerConfig, PrintLoggerFactory}

import scala.concurrent.ExecutionContext.Implicits.global
import scala.io.Source
import scala.sys.process.{Process, ProcessLogger}
import scala.util.Try

class MasterNodeIntegrationSpec extends FlatSpec with LazyLogging with Matchers with BeforeAndAfterAll {

  implicit private val ioTimer: Timer[IO] = IO.timer(global)
  implicit private val ioShift: ContextShift[IO] = IO.contextShift(global)

  private val url = sys.props.get("ethereum.url")
  private val client = EthClient.makeHttpResource[IO](url)

  val dir = new File("../bootstrap")
  def run(cmd: String): Unit = Process(cmd, dir).!(ProcessLogger(_ => ()))
  def runBackground(cmd: String): Unit = Process(cmd, dir).run(ProcessLogger(_ => ()))

  override protected def beforeAll(): Unit = {
    logger.info("bootstrapping npm")
    run("npm install")

    logger.info("starting Ganache")
    runBackground("npm run ganache")

    logger.info("deploying Deployer.sol Ganache")
    run("npm run migrate")
  }

  override protected def afterAll(): Unit = {
    logger.info("killing ganache")
    run("pkill -f ganache")

    logger.info("removing node directories")
    run(s"rm -rf ${nodeDirectory(0)}")
    run(s"rm -rf ${nodeDirectory(1)}")

    logger.info("stopping containers")
    run("docker rm -f 01_node0 01_node1 02_node0")
  }

  "MasterNodes" should "sync their solvers with contract clusters" in {
    PrintLoggerFactory.formatter = new DefaultPrefixFormatter(false, false, false)
    LoggerConfig.factory = PrintLoggerFactory()
    LoggerConfig.level = LogLevel.INFO

    val contractAddress = "0x9995882876ae612bfd829498ccd73dd962ec950a"
    val owner = "0x4180FC65D613bA7E1a385181a219F1DBfE7Bf11d"

    val sttpResource: Resource[IO, SttpBackend[IO, Nothing]] =
      Resource.make(IO(AsyncHttpClientCatsBackend[IO]()))(sttpBackend ⇒ IO(sttpBackend.close()))

    val nodeConfig = DeployerContractConfig(owner, contractAddress)

    EthClient
      .makeHttpResource[IO]()
      .use { ethClient ⇒
        sttpResource.use { implicit sttpBackend ⇒
          for {
            version ← ethClient.clientVersion[IO]()
            _ = logger.info("eth client version {}", version)
            _ = logger.debug("eth config {}", nodeConfig)

            contract = DeployerContract(ethClient, nodeConfig)

            _ <- contract.addAddressToWhitelist[IO](owner)

            pool ← SolversPool[IO]()

            // initializing 0th node: for 2 solvers
            rootPath0 = Paths.get(nodeDirectory(0)).toAbsolutePath
            masterKeys0 = KeysPath(rootPath0.resolve("tendermint").toString)
            _ <- masterKeys0.init
            nodeConfig0 <- NodeConfig.fromArgs(masterKeys0, List("192.168.0.5", "25000", "25002"))
            node0 = MasterNode(masterKeys0, nodeConfig0, contract, pool, rootPath0)

            // initializing 1st node: for 1 solver
            rootPath1 = Paths.get(nodeDirectory(1)).toAbsolutePath
            masterKeys1 = KeysPath(rootPath1.resolve("tendermint").toString)
            _ <- masterKeys1.init
            nodeConfig1 <- NodeConfig.fromArgs(masterKeys1, List("192.168.0.5", "25500", "25501"))
            node1 = MasterNode(masterKeys1, nodeConfig1, contract, pool, rootPath1)

            // registering nodes in contract – nothing should happen here, because no matching work exists
            _ <- contract.addNode[IO](nodeConfig0)
            _ <- contract.addNode[IO](nodeConfig1)

            // adding code – this should cause event, but MasterNode not launched yet, so it wouldn't catch it as event
            _ <- contract.addCode[IO](clusterSize = 2)

            // sending useless tx - just to switch to a new block
            _ <- contract.addAddressToWhitelist[IO](owner)

            // launching MasterNodes - they should take existing cluster info via getNodeClusters
            _ = new Thread(() => node0.run.unsafeRunSync()).start()
            _ = new Thread(() => node1.run.unsafeRunSync()).start()

            // waiting until MasterNodes launched
            _ = Thread.sleep(5000)

            // adding code when MasterNodes launched – both must catch event, but it's for 1st node only
            _ <- contract.addCode[IO](clusterSize = 1)

            // letting MasterNodes to process event and launch solvers
            // then letting solver clusters to make first blocks
            _ = Thread.sleep(20000)

            // gathering solvers' heights from statuses
            cluster1Solver0Status <- heightFromTendermintStatus(nodeConfig0, 0)
            cluster1Solver1Status <- heightFromTendermintStatus(nodeConfig1, 0)
            cluster2Solver0Status <- heightFromTendermintStatus(nodeConfig0, 1)

            // height=2 (consensus for at least 1 block reached) likely means that cluster is configured properly
            _ = cluster1Solver0Status shouldBe Some(2)
            _ = cluster1Solver1Status shouldBe Some(2)
            _ = cluster2Solver0Status shouldBe Some(2)
          } yield ()
        }
      }
      .unsafeRunSync()
  }

  private def nodeDirectory(index: Int): String = System.getProperty("user.home") + s"/.fluence/node$index"

  private def heightFromTendermintStatus(nodeConfig: NodeConfig, solverOrder: Int): IO[Option[Long]] = {
    import io.circe._
    import io.circe.parser._
    val port = nodeConfig.startPort + solverOrder + 100 // +100 corresponds to port mapping scheme from `ClusterData`
    val source = Source.fromURL(s"http://localhost:$port/status").mkString
    val height = parse(source)
      .getOrElse(Json.Null)
      .asObject
      .flatMap(_("result"))
      .flatMap(_.asObject)
      .flatMap(_("sync_info"))
      .flatMap(_.asObject)
      .flatMap(_("latest_block_height"))
      .flatMap(_.asString)
      .flatMap(x => Try(x.toLong).toOption)
    IO.pure(height)
  }
}
