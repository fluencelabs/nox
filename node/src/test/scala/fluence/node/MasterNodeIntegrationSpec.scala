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

import cats.effect._
import com.softwaremill.sttp.SttpBackend
import com.softwaremill.sttp.asynchttpclient.cats.AsyncHttpClientCatsBackend
import fluence.ethclient.EthClient
import fluence.node.docker.{DockerIO, DockerParams}
import fluence.node.eth.{FluenceContract, FluenceContractConfig}
import org.scalatest.{FlatSpec, Matchers}
import slogging.MessageFormatter.DefaultPrefixFormatter
import slogging.{LogLevel, LoggerConfig, PrintLoggerFactory}
import com.softwaremill.sttp._
import com.softwaremill.sttp.circe.asJson

import scala.concurrent.duration._
import scala.language.higherKinds
import scala.sys.process.ProcessLogger

/**
 * This test contains a single test method that checks:
 * - MasterNode connectivity with ganache-hosted Fluence smart contract
 * - MasterNode ability to load previous node clusters and subscribe to new clusters
 * - Successful cluster formation and starting blocks creation
 */
class MasterNodeIntegrationSpec extends FlatSpec with Matchers with Integration {

  "MasterNodes" should "sync their workers with contract clusters" in {
    PrintLoggerFactory.formatter = new DefaultPrefixFormatter(false, false, false)
    LoggerConfig.factory = PrintLoggerFactory()
    LoggerConfig.level = LogLevel.INFO

    val contractAddress = "0x9995882876ae612bfd829498ccd73dd962ec950a"
    val owner = "0x4180FC65D613bA7E1a385181a219F1DBfE7Bf11d"

    logger.info(s"Docker host: '$dockerHost'")

    val sttpResource: Resource[IO, SttpBackend[IO, Nothing]] =
      Resource.make(IO(AsyncHttpClientCatsBackend[IO]()))(sttpBackend ⇒ IO(sttpBackend.close()))

    val contractConfig = FluenceContractConfig(owner, contractAddress)

    def runMaster(portFrom: Short, portTo: Short, name: String, statusPort: Short): IO[String] = {
      DockerIO
        .run[IO](
          DockerParams
            .daemonRun()
            .option("-e", s"TENDERMINT_IP=$dockerHost")
            .option("-e", s"ETHEREUM_IP=$ethereumHost")
            .option("-e", s"PORTS=$portFrom:$portTo")
            .port(statusPort, 5678)
            .option("--name", name)
            .volume("/var/run/docker.sock", "/var/run/docker.sock")
            // statemachine expects wasm binaries in /vmcode folder
            .volume(
              // TODO: by defaults, user.dir in sbt points to a submodule directory while in Idea to the project root
              System.getProperty("user.dir")
                + "/../vm/examples/llamadb/target/wasm32-unknown-unknown/release",
              "/master/vmcode/vmcode-llamadb"
            )
            .image("fluencelabs/node:latest")
        )
        .compile
        .lastOrError
    }

    def getStatus(statusPort: Short)(implicit sttpBackend: SttpBackend[IO, Nothing]): IO[MasterStatus] = {
      import MasterStatus._
      for {
        resp <- sttp.response(asJson[MasterStatus]).get(uri"http://localhost:$statusPort/status").send()
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

    EthClient
      .makeHttpResource[IO]()
      .use { ethClient ⇒
        sttpResource.use { implicit sttpBackend ⇒
          val status1Port: Short = 25400
          val status2Port: Short = 25403
          for {
            master1 <- runMaster(25000, 25002, "master1", status1Port)
            master2 <- runMaster(25003, 25005, "master2", status2Port)

            _ <- eventually[IO](checkMasterRunning(master1), maxWait = 10.seconds)
            _ <- eventually[IO](checkMasterRunning(master2), maxWait = 10.seconds)

            contract = FluenceContract(ethClient, contractConfig)
            status1 <- getStatus(status1Port)
            status2 <- getStatus(status2Port)
            _ <- contract.addNode[IO](status1.nodeConfig).attempt
            _ <- contract.addNode[IO](status2.nodeConfig).attempt
            _ <- contract.addApp[IO]("llamadb", clusterSize = 2)

            _ <- eventually[IO](
              for {
                c1s0 <- heightFromTendermintStatus(25000)
                c1s1 <- heightFromTendermintStatus(25003)
              } yield {
                c1s0 shouldBe Some(2)
                c1s1 shouldBe Some(2)
              },
              maxWait = 90.seconds
            )
          } yield ()
        }
      }
      .unsafeRunSync()
  }

}
