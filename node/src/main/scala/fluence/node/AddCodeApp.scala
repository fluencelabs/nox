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
import fluence.ethclient.EthClient
import fluence.node.eth.{DeployerContract, DeployerContractConfig}
import slogging.MessageFormatter.DefaultPrefixFormatter
import slogging.{LazyLogging, LogLevel, LoggerConfig, PrintLoggerFactory}

object AddCodeApp extends IOApp with LazyLogging {

  /**
   * Adds a single-node llamadb cluster
   */
  override def run(args: List[String]): IO[ExitCode] = {
    configureLogging()
    (for {
      config <- IO.fromEither(
        pureconfig
          .loadConfig[DeployerContractConfig]
          .left
          .map(fs ⇒ new IllegalArgumentException("Can't load or parse configs:" + fs.toString))
      )
    } yield config).attempt.flatMap {
      case Right(config) =>
        // Run master node
        EthClient
          .makeHttpResource[IO]()
          .use { ethClient ⇒
            for {
              version ← ethClient.clientVersion[IO]()
              _ = logger.info("eth client version {}", version)
              _ = logger.debug("eth config {}", config)

              contract = DeployerContract(ethClient, config)

              blockNumber ← contract.addCode[IO]()

              _ = logger.info(s"Code added in block $blockNumber")

            } yield ExitCode.Success
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
