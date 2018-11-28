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

import java.nio.file.{Path, Paths}

import cats.effect.{ExitCode, IO, IOApp, Resource}
import com.softwaremill.sttp.SttpBackend
import com.softwaremill.sttp.asynchttpclient.cats.AsyncHttpClientCatsBackend
import fluence.ethclient.EthClient
import fluence.node.eth.{DeployerContract, DeployerContractConfig}
import fluence.node.solvers.SolversPool
import fluence.node.tendermint.KeysPath
import fluence.swarm.SwarmClient
import slogging.MessageFormatter.DefaultPrefixFormatter
import slogging.{LazyLogging, LogLevel, LoggerConfig, PrintLoggerFactory}
import helpers.ConfigReaderFailuresOps

case class Configuration(
  rootPath: Path,
  masterKeys: KeysPath,
  nodeConfig: NodeConfig,
  contractConfig: DeployerContractConfig,
  swarmClient: Option[SwarmClient[IO]]
)

object MasterNodeApp extends IOApp with LazyLogging {

  private val sttpResource: Resource[IO, SttpBackend[IO, Nothing]] =
    Resource.make(IO(AsyncHttpClientCatsBackend[IO]()))(sttpBackend ⇒ IO(sttpBackend.close()))

  private def configure() =
    for {
      rootPathStr <- IO.fromEither(pureconfig.loadConfig[String]("tendermint-path").left.map(_.asException))
      rootPath = Paths.get(rootPathStr).toAbsolutePath
      masterKeys = KeysPath(rootPath.resolve("tendermint").toString)
      _ ← masterKeys.init
      endpoints <- IO.fromEither(
        pureconfig
          .loadConfig[EndpointsConfig]("endpoints")
          .left
          .map(_.asException)
      )
      solverInfo <- NodeConfig(masterKeys, endpoints)
      config <- IO.fromEither(
        pureconfig
          .loadConfig[DeployerContractConfig]
          .left
          .map(_.asException)
      )
      swarmEnabled <- IO.fromEither(
        pureconfig.loadConfig[Boolean]("use-swarm").left.map(_.asException)
      )
      swarmClient <- if (!swarmEnabled) IO.pure(None)
      else {
        IO.fromEither(pureconfig.loadConfig[String]("swarm.host").left.map(_.asException))
          .map(addr => Some(SwarmClient(addr)))
      }
    } yield Configuration(rootPath, masterKeys, solverInfo, config, swarmClient)

  /**
   * Launches a Master node connecting to ethereum blockchain with Deployer contract.
   *
   */
  override def run(args: List[String]): IO[ExitCode] = {
    configureLogging()
    configure().attempt.flatMap {
      case Right(Configuration(rootPath, masterKeys, nodeConfig, config, swarmClient)) =>
        // Run master node
        EthClient
          .makeHttpResource[IO]()
          .use { ethClient ⇒
            sttpResource.use { implicit sttpBackend ⇒
              for {
                version ← ethClient.clientVersion[IO]()
                _ = logger.info("eth client version {}", version)
                _ = logger.debug("eth config {}", config)

                contract = DeployerContract(ethClient, config)

                // TODO: should check that node is registered, but should not send transactions
                _ <- contract
                  .addAddressToWhitelist[IO](config.deployerContractOwnerAccount)
                  .attempt
                  .map(r ⇒ logger.debug(s"Whitelisting address: $r"))
                _ <- contract
                  .addNode[IO](nodeConfig)
                  .attempt
                  .map(r ⇒ logger.debug(s"Adding node: $r"))

                pool ← SolversPool[IO]()

                node = MasterNode(masterKeys, nodeConfig, contract, pool, swarmClient, rootPath)

                result ← node.run

              } yield result
            }
          }
      case Left(value) =>
        logger.error("Error: {}", value)
        IO.pure(ExitCode.Error)
    }
  }

  private def configureLogging(): Unit = {
    PrintLoggerFactory.formatter = new DefaultPrefixFormatter(false, false, false)
    LoggerConfig.factory = PrintLoggerFactory()
    LoggerConfig.level = LogLevel.DEBUG
  }
}
