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
import java.nio.file.{Files, Path, Paths}

import cats.effect._
import cats.syntax.applicativeError._
import cats.syntax.functor._
import cats.syntax.monadError._
import com.softwaremill.sttp.SttpBackend
import com.softwaremill.sttp.asynchttpclient.cats.AsyncHttpClientCatsBackend
import fluence.ethclient.EthClient
import fluence.node.eth.{DeployerContract, DeployerContractConfig}
import fluence.node.solvers.{SolversPool, TestCodeManager}
import fluence.node.tendermint.KeysPath
import org.scalactic.source.Position
import org.scalatest.exceptions.{TestFailedDueToTimeoutException, TestFailedException}
import org.scalatest.time.Span
import org.scalatest.{BeforeAndAfterAll, FlatSpec, Matchers}
import slogging.MessageFormatter.DefaultPrefixFormatter
import slogging.{LazyLogging, LogLevel, LoggerConfig, PrintLoggerFactory}

import scala.concurrent.ExecutionContext.Implicits.global
import scala.concurrent.duration._
import scala.io.Source
import scala.language.higherKinds
import scala.sys.process.{Process, ProcessLogger}
import scala.util.Try

/**
 * This test contains a single test method that checks:
 * - MasterNode connectivity with ganache-hosted Deployer smart contract
 * - MasterNode ability to load previous node clusters and subscribe to new clusters
 * - Successful cluster formation and starting blocks creation
 */
class MasterNodeIntegrationSpec extends FlatSpec with LazyLogging with Matchers with BeforeAndAfterAll {

  implicit private val ioTimer: Timer[IO] = IO.timer(global)
  implicit private val ioShift: ContextShift[IO] = IO.contextShift(global)

  private val url = sys.props.get("ethereum.url")
  private val client = EthClient.makeHttpResource[IO](url)

  val bootstrapDir = new File("../bootstrap")
  def run(cmd: String): Unit = Process(cmd, bootstrapDir).!(ProcessLogger(_ => ()))
  def runBackground(cmd: String): Unit = Process(cmd, bootstrapDir).run(ProcessLogger(_ => ()))

  override protected def beforeAll(): Unit = {
    logger.info("bootstrapping npm")
    run("npm install")

    logger.info("starting Ganache")
    runBackground("npm run ganache > /dev/null")

    logger.info("deploying Deployer.sol Ganache")
    run("npm run migrate")

    Files.createDirectories(solversPath(0))
    Files.createDirectories(solversPath(1))
    Files.createDirectories(keysPath(0))
    Files.createDirectories(keysPath(1))
  }

  override protected def afterAll(): Unit = {
    logger.info("killing ganache")
    run("pkill -f ganache")

    logger.info("clearing node directories from containers")
    for (subpath <- List("keys", "solvers"))
      for (i <- 0 to 1)
        run(
          s"docker run --rm -i -v ${nodePath(i)}:/node --entrypoint rm fluencelabs/solver:latest -rf /node/$subpath"
        )

    logger.info("removing node directories")
    run(s"rm -rf ${nodePath(0)}")
    run(s"rm -rf ${nodePath(1)}")

    logger.info("stopping containers")
    run("docker rm -f 01_node0 01_node1 02_node0")
  }

  "MasterNodes" should "sync their solvers with contract clusters" in {
    PrintLoggerFactory.formatter = new DefaultPrefixFormatter(false, false, false)
    LoggerConfig.factory = PrintLoggerFactory()
    LoggerConfig.level = LogLevel.INFO

    val contractAddress = "0x9995882876ae612bfd829498ccd73dd962ec950a"
    val owner = "0x4180FC65D613bA7E1a385181a219F1DBfE7Bf11d"

    val dockerHostIP = getOS match {
      case "linux" => detectIPStringByNetworkInterface("docker0")
      case "mac" => detectIPStringByNetworkInterface("en0")
      case _ => throw new RuntimeException("The test doesn't support this OS")
    }
    logger.info(s"Docker host: '$dockerHostIP'")

    val codeManager = new TestCodeManager[IO]

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
            masterKeys0 = KeysPath(keysPath(0).toString)
            _ <- masterKeys0.init
            nodeConfig0 <- NodeConfig(masterKeys0, EndpointsConfig(dockerHostIP, 25000, 25002))
            node0 = MasterNode(masterKeys0, nodeConfig0, contract, pool, codeManager, solversPath(0))

            // initializing 1st node: for 1 solver
            masterKeys1 = KeysPath(keysPath(1).toString)
            _ <- masterKeys1.init
            nodeConfig1 <- NodeConfig(masterKeys1, EndpointsConfig(dockerHostIP, 25500, 25501))
            node1 = MasterNode(masterKeys1, nodeConfig1, contract, pool, codeManager, solversPath(1))

            // registering nodes in contract – nothing should happen here, because no matching work exists
            _ <- contract.addNode[IO](nodeConfig0)
            _ <- contract.addNode[IO](nodeConfig1)

            // adding code – this should cause event, but MasterNodes not launched yet, so they wouldn't catch it
            _ <- contract.addCode[IO](clusterSize = 2)

            // sending useless tx - just to switch to a new block
            _ <- contract.addAddressToWhitelist[IO](owner)

            // launching MasterNodes - they should take existing cluster info via getNodeClusters
            _ <- Concurrent[IO].start(node0.run) // TODO: stop cluster & join fiber
            _ <- Concurrent[IO].start(node1.run) // TODO: stop cluster & join fiber

            // waiting until MasterNodes launched
            _ <- eventually[IO](
              {
                val alive: MasterNode => IO[Boolean] = _.pool.healths.map(_.exists { case (_, h) => h.isHealthy })
                for {
                  alive0 <- alive(node0)
                  alive1 <- alive(node1)
                } yield {
                  alive0 shouldBe true
                  alive1 shouldBe true
                }
              },
              maxWait = 120.seconds // TODO: fix startup time
            )

            // adding code when MasterNodes launched – both must catch event, but it's for 1st node only
            _ <- contract.addCode[IO](clusterSize = 1)

            // letting MasterNodes to process event and launch solvers
            // then letting solver clusters to make first blocks
            _ <- eventually[IO](
              for {
                c1s0 <- heightFromTendermintStatus(nodeConfig0, 0)
                c1s1 <- heightFromTendermintStatus(nodeConfig1, 0)
                c2s0 <- heightFromTendermintStatus(nodeConfig0, 1)
              } yield {
                c1s0 shouldBe Some(2)
                c1s1 shouldBe Some(2)
                c2s0 shouldBe Some(2)
              },
              maxWait = 30.seconds
            )
          } yield ()
        }
      }
      .unsafeRunSync()
  }

  private def eventually[F[_]: Sync: Timer](
    p: => F[Unit],
    period: FiniteDuration = 1.second,
    maxWait: FiniteDuration = 10.seconds
  )(implicit pos: Position): F[_] = {
    fs2.Stream
      .awakeEvery[F](period)
      .take((maxWait / period).toLong)
      .evalMap(_ => p.attempt)
      .takeThrough(_.isLeft) // until p returns Right(Unit)
      .compile
      .last
      .map {
        case Some(Right(_)) =>
        case Some(Left(e)) => throw e
        case _ => throw new RuntimeException(s"eventually timed out after $maxWait")
      }
      .adaptError {
        case e: TestFailedException =>
          e.modifyMessage(m => Some(s"eventually timed out after $maxWait" + m.map(": " + _).getOrElse("")))
        case e =>
          new TestFailedDueToTimeoutException(
            _ => Some(s"eventually timed out after $maxWait" + Option(e.getMessage).map(": " + _).getOrElse("")),
            Some(e),
            pos,
            None,
            Span.convertDurationToSpan(maxWait)
          )
      }
  }

  private def nodePath(index: Int): Path = Paths.get(System.getProperty("user.home") + s"/.fluence/node$index")

  private def solversPath(index: Int): Path = nodePath(index).resolve("solvers")

  private def keysPath(index: Int): Path = nodePath(index).resolve("keys")

  private def heightFromTendermintStatus(nodeConfig: NodeConfig, solverOrder: Int): IO[Option[Long]] = IO {
    import io.circe._
    import io.circe.parser._
    val port = nodeConfig.endpoints.minPort + solverOrder + 100 // +100 corresponds to port mapping scheme from `ClusterData`
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
    height
  }

  private def detectIPStringByNetworkInterface(interface: String): InetAddress = {
    import sys.process._
    val ifconfigCmd = Seq("ifconfig", interface)
    val grepCmd = Seq("grep", "inet ")
    val awkCmd = Seq("awk", "{print $2}")
    InetAddress.getByName((ifconfigCmd #| grepCmd #| awkCmd).!!.replaceAll("[^0-9\\.]", ""))
  }

  private def getOS: String = {
    // TODO: should use more comprehensive and reliable OS detection
    val osName = System.getProperty("os.name").toLowerCase()
    if (osName.contains("windows"))
      "windows"
    else if (osName.contains("mac") || osName.contains("darwin"))
      "mac"
    else
      "linux"
  }
}
