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
import com.softwaremill.sttp.SttpBackend
import com.softwaremill.sttp.asynchttpclient.cats.AsyncHttpClientCatsBackend
import fluence.ethclient.EthClient
import fluence.node.docker.{DockerIO, DockerParams}
import fluence.node.eth.{DeployerContract, DeployerContractConfig}
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

  val bootstrapDir = new File("../bootstrap")
  def run(cmd: String): Unit = Process(cmd, bootstrapDir).!(ProcessLogger(_ => ()))
  def runBackground(cmd: String): Unit = Process(cmd, bootstrapDir).run(ProcessLogger(_ => ()))

  private val dockerHost = getOS match {
    case "linux" => ifaceIP("docker0")
    case "mac" => "host.docker.internal" //ifaceIP("en0")
    case _ => throw new RuntimeException("The test doesn't support this OS")
  }

  override protected def beforeAll(): Unit = {
    logger.info("bootstrapping npm")
    run("npm install")

    logger.info("starting Ganache")
    runBackground("npm run ganache > /dev/null")

    logger.info("deploying Deployer.sol Ganache")
    run("npm run migrate")
  }

  override protected def afterAll(): Unit = {
    logger.info("killing ganache")
    run("pkill -f ganache")

    logger.info("stopping containers")
    // TODO: kill containers through Master's HTTP API
    run("docker rm -f 01_node0 01_node1 02_node0 master1 master2")
  }

  "MasterNodes" should "sync their solvers with contract clusters" in {
    PrintLoggerFactory.formatter = new DefaultPrefixFormatter(false, false, false)
    LoggerConfig.factory = PrintLoggerFactory()
    LoggerConfig.level = LogLevel.INFO

    val contractAddress = "0x9995882876ae612bfd829498ccd73dd962ec950a"
    val owner = "0x4180FC65D613bA7E1a385181a219F1DBfE7Bf11d"

    logger.info(s"Docker host: '$dockerHost'")

    val sttpResource: Resource[IO, SttpBackend[IO, Nothing]] =
      Resource.make(IO(AsyncHttpClientCatsBackend[IO]()))(sttpBackend ⇒ IO(sttpBackend.close()))

    val contractConfig = DeployerContractConfig(owner, contractAddress)

    def runMaster(portFrom: Int, portTo: Int, name: String): IO[String] = {
      DockerIO
        .run[IO](
          DockerParams
            .daemonRun()
            .option("-e", s"TENDERMINT_IP=$dockerHost")
            .option("-e", s"PORTS=$portFrom:$portTo")
            .option("--name", name)
            .volume("/var/run/docker.sock", "/var/run/docker.sock")
            .image("fluencelabs/node")
        )
        .compile
        .lastOrError
    }

    //TODO: change check to Master's HTTP API
    def checkMasterRunning(containerId: String): IO[Unit] = {
      IO {
        var line = ""
        scala.sys.process
          .Process(s"docker logs $containerId")
          .!!(ProcessLogger(_ => {}, o => line = o))
        line
      }.map(line => line should include("switching to the new clusters"))
    }

    EthClient
      .makeHttpResource[IO]()
      .use { ethClient ⇒
        sttpResource.use { implicit sttpBackend ⇒
          for {
            master1 <- runMaster(25000, 25002, "master1")
            master2 <- runMaster(25003, 25005, "master2")

            _ <- eventually[IO](checkMasterRunning(master1))
            _ <- eventually[IO](checkMasterRunning(master2))

            contract = DeployerContract(ethClient, contractConfig)
            _ <- contract.addCode[IO](clusterSize = 2)

            _ <- eventually[IO](
              for {
                c1s0 <- heightFromTendermintStatus(25000)
                c1s1 <- heightFromTendermintStatus(25003)
              } yield {
                c1s0 shouldBe Some(2)
                c1s1 shouldBe Some(2)
              },
              maxWait = 30.seconds
            )
          } yield {}
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

  private def heightFromTendermintStatus(startPort: Int): IO[Option[Long]] = IO {
    import io.circe._
    import io.circe.parser._
    val port = startPort + 100 // +100 corresponds to port mapping scheme from `ClusterData`
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

  // return IP address of the `interface`
  private def ifaceIP(interface: String): String = {
    import sys.process._
    val ifconfigCmd = Seq("ifconfig", interface)
    val grepCmd = Seq("grep", "inet ")
    val awkCmd = Seq("awk", "{print $2}")
    InetAddress.getByName((ifconfigCmd #| grepCmd #| awkCmd).!!.replaceAll("[^0-9\\.]", "")).getHostAddress
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
