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

import cats.effect.ExitCase.{Canceled, Completed, Error}
import cats.effect._
import com.softwaremill.sttp.SttpBackend
import com.softwaremill.sttp.asynchttpclient.cats.AsyncHttpClientCatsBackend
import fluence.node.config.{Configuration, MasterConfig}
import fluence.node.status.StatusAggregator
import slogging.MessageFormatter.DefaultPrefixFormatter
import slogging.{LazyLogging, LogLevel, LoggerConfig, PrintLoggerFactory}

object MasterNodeApp extends IOApp with LazyLogging {

  private val sttpResource: Resource[IO, SttpBackend[IO, Nothing]] =
    Resource.make(IO(AsyncHttpClientCatsBackend[IO]()))(sttpBackend ⇒ IO(sttpBackend.close()))

  /**
   * Launches a Master Node instance
   * Assuming to be launched inside Docker image
   *
   * - Adds contractOwnerAccount to whitelist
   * - Starts to listen Ethereum for ClusterFormed event
   * - On ClusterFormed event, launches Worker Docker container
   * - Starts HTTP API serving status information
   */
  override def run(args: List[String]): IO[ExitCode] = {
    configureLogging()
    MasterConfig
      .load()
      .flatMap(mc ⇒ Configuration.init(mc).map(_ → mc))
      .flatMap {
        case (conf, masterConf) =>
          // Run master node and status server
          sttpResource
            .flatMap(implicit sttpBackend ⇒ MasterNode.make[IO, IO.Par](masterConf, conf.nodeConfig, conf.rootPath))
            .use { node ⇒
              logger.debug("eth config {}", masterConf.contract)

              StatusAggregator
                .makeHttpResource(masterConf, node)
                .use { status =>
                  logger.info("Status server has started on: " + status.address)
                  node.run
                }
            }
      }
      .attempt
      .flatMap(IO.fromEither)
      .guaranteeCase {
        case Canceled =>
          IO(logger.error("MasterNodeApp was canceled"))
        case Error(e) =>
          IO(logger.error("MasterNodeApp stopped with error: {}", e)).map(_ => e.printStackTrace(System.err))
        case Completed =>
          IO(logger.info("MasterNodeApp exited gracefully"))
      }
  }

  private def configureLogging(): Unit = {
    PrintLoggerFactory.formatter =
      new DefaultPrefixFormatter(printLevel = true, printName = true, printTimestamp = true)
    LoggerConfig.factory = PrintLoggerFactory()
    LoggerConfig.level = LogLevel.DEBUG
  }
}
