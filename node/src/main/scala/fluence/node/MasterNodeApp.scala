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

import java.nio.ByteBuffer

import cats.data.EitherT
import cats.effect.ExitCase.{Canceled, Completed, Error}
import cats.effect._
import cats.syntax.apply._
import cats.syntax.flatMap._
import com.softwaremill.sttp.SttpBackend
import fluence.EitherTSttpBackend
import fluence.crypto.KeyPair
import fluence.crypto.ecdsa.Ecdsa
import fluence.crypto.signature.SignAlgo
import fluence.effects.docker.DockerIO
import fluence.kad.KademliaConf
import fluence.kad.http.UriContact
import fluence.log.{Log, LogFactory}
import fluence.node.config.{Configuration, MasterConfig}
import fluence.node.status.StatusAggregator
import fluence.node.workers.DockerWorkersPool
import slogging.MessageFormatter.DefaultPrefixFormatter
import slogging.{LazyLogging, LogLevel, LoggerConfig, PrintLoggerFactory}

import scala.concurrent.duration._
import scala.language.higherKinds

object MasterNodeApp extends IOApp with LazyLogging {
  private val sttpResource: Resource[IO, SttpBackend[EitherT[IO, Throwable, ?], fs2.Stream[IO, ByteBuffer]]] =
    Resource.make(IO(EitherTSttpBackend[IO]()))(sttpBackend ⇒ IO(sttpBackend.close()))

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
    type STTP = SttpBackend[EitherT[IO, Throwable, ?], fs2.Stream[IO, ByteBuffer]]

    implicit val logFactory: LogFactory[IO] = LogFactory.forChains[IO]()

    MasterConfig
      .load()
      .map(mc => { configureLogging(mc.logLevel); mc })
      .flatMap { masterConf =>
        // Run master node and status server
        (for {
          implicit0(sttp: STTP) <- sttpResource
          implicit0(dockerIO: DockerIO[IO]) <- DockerIO.make[IO]()
          implicit0(log: Log[IO]) ← Resource.liftF(logFactory.init("masterApp", "run"))
          conf <- Resource.liftF(Configuration.init[IO](masterConf))
          pool <- DockerWorkersPool.make(
            masterConf.ports.minPort,
            masterConf.ports.maxPort,
            conf.rootPath,
            masterConf.remoteStorage
          )
          kad ← KademliaNode.make[IO, IO.Par](
            // TODO use real publicly available host and port
            "localhost",
            6565,
            KademliaConf(16, 16, 4, 1.minute), // TODO read from conf
            Ecdsa.signAlgo, // TODO use tendermint algo
            Ecdsa.ecdsa_secp256k1_sha256.generateKeyPair.unsafe(None) // TODO use tendermint validator key
          )
          node <- MasterNode.make[IO, IO.Par, UriContact](masterConf, conf.nodeConfig, pool, kad.kademlia)
        } yield (kad.http, node, log)).use {
          case (kadHttp, node, log) ⇒
            logger.debug("eth config {}", masterConf.contract)

            (for {
              st ← StatusAggregator.make(masterConf, node)
              server ← MasterHttp.make[IO, IO.Par, UriContact](
                "0.0.0.0",
                masterConf.httpApi.port.toShort,
                st,
                node.pool,
                kadHttp
              )
            } yield server).use { server =>
              log.info("Http api server has started on: " + server.address) *>
                node.run
            }
        }

      }
      .guaranteeCase {
        case Canceled =>
          IO(logger.error("MasterNodeApp was canceled"))
        case Error(e) =>
          IO(logger.error("MasterNodeApp stopped with error: {}", e)).map(_ => e.printStackTrace(System.err))
        case Completed =>
          IO(logger.info("MasterNodeApp exited gracefully"))
      }
  }

  private def configureLogging(level: LogLevel): Unit = {
    PrintLoggerFactory.formatter =
      new DefaultPrefixFormatter(printLevel = true, printName = true, printTimestamp = true)
    LoggerConfig.factory = PrintLoggerFactory()
    LoggerConfig.level = level
  }
}
