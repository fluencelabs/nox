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

import cats.effect.{ExitCode, IO, IOApp, Resource}
import com.softwaremill.sttp.SttpBackend
import com.softwaremill.sttp.asynchttpclient.cats.AsyncHttpClientCatsBackend
import fluence.ethclient.EthClient
import fluence.node.eth.DeployerContract
import fluence.node.solvers.SolversPool
import slogging.MessageFormatter.DefaultPrefixFormatter
import slogging.{LazyLogging, LogLevel, LoggerConfig, PrintLoggerFactory}

object MasterNodeApp extends IOApp with LazyLogging {

  private val sttpResource: Resource[IO, SttpBackend[IO, Nothing]] =
    Resource.make(IO(AsyncHttpClientCatsBackend[IO]()))(sttpBackend ⇒ IO(sttpBackend.close()))

  /**
   * Launches a Master node connecting to ethereum blockchain with Deployer contract.
   *
   */
  override def run(args: List[String]): IO[ExitCode] = {
    configureLogging()
    Configuration.configure().attempt.flatMap {
      case Right(Configuration(rootPath, nodeConfig, config, swarmEnabled, ethereumRPC, masterNodeContainerId)) =>
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

                codeManager <- Configuration.getCodeManager(swarmEnabled)

                node = MasterNode(nodeConfig, contract, pool, codeManager, rootPath, masterNodeContainerId)

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
