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

import java.nio.file.{Files, Path, Paths}

import cats.effect.{ExitCode, IO, IOApp, Resource}
import com.softwaremill.sttp.SttpBackend
import com.softwaremill.sttp.asynchttpclient.cats.AsyncHttpClientCatsBackend
import com.typesafe.config.Config
import fluence.ethclient.EthClient
import fluence.node.ConfigOps._
import fluence.node.eth.{DeployerContract, DeployerContractConfig, EthereumRPCConfig}
import fluence.node.solvers.{CodeManager, SolversPool, SwarmCodeManager, TestCodeManager}
import fluence.node.tendermint.KeysPath
import fluence.swarm.SwarmClient
import pureconfig.backend.ConfigFactoryWrapper
import pureconfig.error.ConfigReaderFailures
import slogging.MessageFormatter.DefaultPrefixFormatter
import slogging.{LazyLogging, LogLevel, LoggerConfig, PrintLoggerFactory}

case class Configuration(
  rootPath: Path,
  masterKeys: KeysPath,
  nodeConfig: NodeConfig,
  contractConfig: DeployerContractConfig,
  swarmEnabled: Boolean,
  ethereumRPC: EthereumRPCConfig
)

object MasterNodeApp extends IOApp with LazyLogging {

  private val sttpResource: Resource[IO, SttpBackend[IO, Nothing]] =
    Resource.make(IO(AsyncHttpClientCatsBackend[IO]()))(sttpBackend ⇒ IO(sttpBackend.close()))

  def loadConfig(): Either[ConfigReaderFailures, Config] = {
    import ConfigFactoryWrapper._
    val containerConfig = "/master/application.conf"

    loadFile(Paths.get(containerConfig)) match {
      case Left(e) =>
        logger.info(s"unable to load $containerConfig: $e") // how to interpret ConfigReaderFailures?
        load()
      case Right(config) =>
        load.map(config.withFallback)
    }
  }

  def initKeys(rootPath: Path): IO[KeysPath] = {
    val keysPath = rootPath.resolve("tendermint")
    val masterKeys = KeysPath(keysPath.toString)
    IO(Files.createDirectories(keysPath)).flatMap(_ => masterKeys.init.map(_ => masterKeys))
  }

  private def configure(): IO[Configuration] =
    for {
      config <- loadConfig().toIO

      rootPathStr <- pureconfig.loadConfig[String](config, "tendermint-path").toIO
      rootPath = Paths.get(rootPathStr).toAbsolutePath

      masterKeys <- initKeys(rootPath)

      endpoints <- pureconfig.loadConfig[EndpointsConfig](config, "endpoints").toIO
      solverInfo <- NodeConfig(masterKeys, endpoints)

      contractConfig <- pureconfig.loadConfig[DeployerContractConfig](config).toIO
      swarmEnabled <- pureconfig.loadConfig[Boolean](config, "use-swarm").toIO

      ethereumRPC <- pureconfig.loadConfig[EthereumRPCConfig](config, "ethereum").toIO

    } yield Configuration(rootPath, masterKeys, solverInfo, contractConfig, swarmEnabled, ethereumRPC)

  private def getCodeManager(
    swarmEnabled: Boolean
  )(implicit sttpBackend: SttpBackend[IO, Nothing]): IO[CodeManager[IO]] = {
    if (!swarmEnabled) IO(new TestCodeManager[IO]())
    else {
      pureconfig
        .loadConfig[String]("swarm.host")
        .toIO
        .flatMap(addr => SwarmClient(addr))
        .map(client => new SwarmCodeManager[IO](client))
    }
  }

  /**
   * Launches a Master node connecting to ethereum blockchain with Deployer contract.
   *
   */
  override def run(args: List[String]): IO[ExitCode] = {
    configureLogging()
    configure().attempt.flatMap {
      case Right(Configuration(rootPath, masterKeys, nodeConfig, config, swarmEnabled, ethereumRPC)) =>
        // Run master node
        EthClient
          .makeHttpResource[IO](Some(ethereumRPC.uri))
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

                codeManager <- getCodeManager(swarmEnabled)

                node = MasterNode(masterKeys, nodeConfig, contract, pool, codeManager, rootPath)

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
