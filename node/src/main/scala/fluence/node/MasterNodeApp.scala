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
import fluence.node.eth.FluenceContract
import fluence.node.solvers.{CodeManager, SolversPool, SwarmCodeManager, TestCodeManager}
import fluence.swarm.SwarmClient
import slogging.MessageFormatter.DefaultPrefixFormatter
import slogging.{LazyLogging, LogLevel, LoggerConfig, PrintLoggerFactory}
import fluence.node.config.SwarmConfig
import scala.concurrent.duration.MILLISECONDS

object MasterNodeApp extends IOApp with LazyLogging {

  private val sttpResource: Resource[IO, SttpBackend[IO, Nothing]] =
    Resource.make(IO(AsyncHttpClientCatsBackend[IO]()))(sttpBackend ⇒ IO(sttpBackend.close()))

  private def getCodeManager(
    config: Option[SwarmConfig]
  )(implicit sttpBackend: SttpBackend[IO, Nothing]): IO[CodeManager[IO]] =
    config match {
      case Some(c) =>
        SwarmClient(c.host)
          .map(client => new SwarmCodeManager[IO](client))
      case None =>
        IO(new TestCodeManager[IO]())
    }

  /**
   * Launches a Master Node instance
   * Assuming to be launched inside Docker image
   *
   * - Adds contractOwnerAccount to whitelist
   * - Starts to listen Ethereum for ClusterFormed event
   * - On ClusterFormed event, launches Solver Docker container
   * - Starts HTTP API serving status information
   */
  override def run(args: List[String]): IO[ExitCode] = {
    configureLogging()
    Configuration
      .create()
      .flatMap {
        case (rawConfig, configuration) =>
          import configuration._
          // Run master node and status server
          val resources =
            for {
              ethClientResource <- EthClient.makeHttpResource[IO](Some(ethereumRPCConfig.uri))
              sttpBackend <- sttpResource
            } yield (ethClientResource, sttpBackend)

          resources.use {
            // Type annotations are here to make IDEA's type inference happy
            case (ethClient: EthClient, sttpBackend: SttpBackend[IO, Nothing]) ⇒
              implicit val backend: SttpBackend[IO, Nothing] = sttpBackend
              for {
                version ← ethClient.clientVersion[IO]()
                _ = logger.info("eth client version {}", version)
                _ = logger.debug("eth config {}", contractConfig)

                contract = FluenceContract(ethClient, contractConfig)

                // TODO: should check that node is registered, but should not send transactions
                _ <- contract
                  .addAddressToWhitelist[IO](contractConfig.ownerAccount)
                  .attempt
                  .map(r ⇒ logger.debug(s"Whitelisting address: $r"))
                _ <- contract
                  .addNode[IO](nodeConfig)
                  .attempt
                  .map(r ⇒ logger.debug(s"Adding node: $r"))

                pool ← SolversPool[IO]()

                codeManager <- getCodeManager(swarmConfig)

                node = MasterNode(nodeConfig, contract, pool, codeManager, rootPath, masterContainerId)

                currentTime <- timer.clock.monotonic(MILLISECONDS)
                result <- StatusAggregator.makeHttpResource(statsServerConfig, rawConfig, node, currentTime).use {
                  status =>
                    logger.info("Status server has started on: " + status.address)
                    node.run
                }
              } yield result
          }
      }
      .attempt
      .flatMap {
        case Left(err) =>
          logger.error("Error: {}", err)
          IO.pure(ExitCode.Error)
        case Right(ec) => IO.pure(ec)
      }
  }

  private def configureLogging(): Unit = {
    PrintLoggerFactory.formatter = new DefaultPrefixFormatter(false, false, false)
    LoggerConfig.factory = PrintLoggerFactory()
    LoggerConfig.level = LogLevel.DEBUG
  }
}
