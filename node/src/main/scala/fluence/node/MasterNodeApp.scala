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
import cats.syntax.functor._
import com.softwaremill.sttp.SttpBackend
import com.softwaremill.sttp.asynchttpclient.cats.AsyncHttpClientCatsBackend
import fluence.ethclient.EthClient
import fluence.node.eth.DeployerContract
import fluence.node.solvers.{CodeManager, SolversPool, SwarmCodeManager, TestCodeManager}
import fluence.swarm.SwarmClient
import slogging.MessageFormatter.DefaultPrefixFormatter
import slogging.{LazyLogging, LogLevel, LoggerConfig, PrintLoggerFactory}
import fluence.node.config.{MasterConfig, SwarmConfig}

object MasterNodeApp extends IOApp with LazyLogging {

  private val sttpResource: Resource[IO, SttpBackend[IO, Nothing]] =
    Resource.make(IO(AsyncHttpClientCatsBackend[IO]()))(sttpBackend ⇒ IO(sttpBackend.close()))

  private def getCodeManager(
    config: Option[SwarmConfig]
  )(implicit sttpBackend: SttpBackend[IO, Nothing]): IO[CodeManager[IO]] = {
    config.map { c =>
      SwarmClient(c.host)
        .map(client => new SwarmCodeManager[IO](client))
    }.getOrElse(IO(new TestCodeManager[IO]()))
  }

  /**
   * Launches a Master node connecting to ethereum blockchain with Deployer contract.
   *
   */
  override def run(args: List[String]): IO[ExitCode] = {
    configureLogging()
    Configuration
      .create()
      .flatMap {
        case (
            rawConfig,
            Configuration(
              rootPath,
              nodeConfig,
              deployerConfig,
              swarmConfigOpt,
              statServer,
              ethereumRPC,
              masterNodeContainerId
            )
            ) =>
          // Run master node and status server
          val resources = for {
            ethClientResource <- EthClient.makeHttpResource[IO](Some(ethereumRPC.uri))
            sttpBackend <- sttpResource
          } yield (ethClientResource, sttpBackend)

          resources.use {
            case (ethClient, sttpBackend) ⇒
              implicit val backend: SttpBackend[IO, Nothing] = sttpBackend
              for {
                version ← ethClient.clientVersion[IO]()
                _ = logger.info("eth client version {}", version)
                _ = logger.debug("eth config {}", deployerConfig)

                contract = DeployerContract(ethClient, deployerConfig)

                // TODO: should check that node is registered, but should not send transactions
                _ <- contract
                  .addAddressToWhitelist[IO](deployerConfig.deployerContractOwnerAccount)
                  .attempt
                  .map(r ⇒ logger.debug(s"Whitelisting address: $r"))
                _ <- contract
                  .addNode[IO](nodeConfig)
                  .attempt
                  .map(r ⇒ logger.debug(s"Adding node: $r"))

                pool ← SolversPool[IO]()

                codeManager <- getCodeManager(swarmConfigOpt)

                node = MasterNode(nodeConfig, contract, pool, codeManager, rootPath, masterNodeContainerId)

                result <- StateManager.makeResource(statServer, rawConfig, node).use { status =>
                  node.run
                }
              } yield result
          }
      }
      .handleErrorWith { err =>
        logger.error("Error: {}", err)
        IO.pure(ExitCode.Error)
      }
  }

  private def configureLogging(): Unit = {
    PrintLoggerFactory.formatter = new DefaultPrefixFormatter(false, false, false)
    LoggerConfig.factory = PrintLoggerFactory()
    LoggerConfig.level = LogLevel.DEBUG
  }
}
