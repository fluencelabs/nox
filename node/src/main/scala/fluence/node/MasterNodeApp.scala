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

import cats.effect.{ExitCode, IO, IOApp}
import com.softwaremill.sttp.SttpBackend
import com.softwaremill.sttp.asynchttpclient.cats.AsyncHttpClientCatsBackend
import fluence.ethclient.EthClient
import fluence.node.eth.{DeployerContract, DeployerContractConfig}
import fluence.node.solvers.SolversPool
import fluence.node.tendermint.KeysPath
import slogging.MessageFormatter.DefaultPrefixFormatter
import slogging.{LazyLogging, LogLevel, LoggerConfig, PrintLoggerFactory}

object MasterNodeApp extends IOApp with LazyLogging {

  /**
   * Launches a single solver connecting to ethereum blockchain with Deployer contract.
   *
   * @param args 1st: Tendermint key location. 2nd: Tendermint p2p host IP. 3rd and 4th: Tendermint p2p port range.
   */
  override def run(args: List[String]): IO[ExitCode] = {
    configureLogging()
    val tendermintMasterKeys :: restArgs = args
    val masterKeys = KeysPath(tendermintMasterKeys)

    implicit val sttpBackend: SttpBackend[IO, Nothing] = AsyncHttpClientCatsBackend[IO]()

    (for {
      solverInfo <- NodeConfig.fromArgs(masterKeys, restArgs)
      config <- IO.fromEither(
        pureconfig
          .loadConfig[DeployerContractConfig]
          .left
          .map(fs ⇒ new IllegalArgumentException(fs.toString))
      )
    } yield (solverInfo, config)).attempt.flatMap {
      case Right((nodeConfig, config)) =>
        // Run master node
        EthClient
          .makeHttpResource[IO]()
          .use { ethClient ⇒
            for {
              version ← ethClient.clientVersion[IO]()
              _ = logger.info("eth client version {}", version)
              _ = logger.debug("eth config {}", config)

              contract = DeployerContract(ethClient, config)

              _ <- contract.addNode[IO](nodeConfig)

              pool ← SolversPool[IO]()

              node = MasterNode(masterKeys, nodeConfig, contract, pool)

              result ← node.run

            } yield result
          }
      case Left(value) =>
        logger.error("Error: {}", value)
        IO.pure(ExitCode.Error)
    }
  }

  private def configureLogging(): Unit = {
    PrintLoggerFactory.formatter = new DefaultPrefixFormatter(false, false, false)
    LoggerConfig.factory = PrintLoggerFactory()
    LoggerConfig.level = LogLevel.INFO
  }
}
