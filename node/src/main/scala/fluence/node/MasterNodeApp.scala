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
import java.nio.file.Path

import cats.data.{EitherT, NonEmptyList}
import cats.effect.ExitCase.{Canceled, Completed, Error}
import cats.effect._
import cats.effect.concurrent.Ref
import cats.syntax.apply._
import cats.syntax.flatMap._
import com.softwaremill.sttp.SttpBackend
import fluence.EitherTSttpBackend
import fluence.crypto.eddsa.Ed25519
import fluence.effects.{Backoff, EffectError}
import fluence.effects.docker.DockerIO
import fluence.effects.ipfs.IpfsUploader
import fluence.kad.conf.KademliaConfig
import fluence.kad.contact.UriContact
import fluence.kad.http.{KademliaHttp, KademliaHttpNode}
import fluence.log.{Log, LogFactory}
import fluence.node.config.storage.RemoteStorageConfig
import fluence.node.config.{Configuration, HttpApiConfig, MasterConfig}
import fluence.node.status.StatusAggregator
import fluence.node.workers.DockerWorkersPool
import fluence.node.workers.subscription.ResponseSubscriberImpl
import fluence.node.workers.tendermint.BlockUploading

import scala.language.higherKinds

object MasterNodeApp extends IOApp {
  type STTP = SttpBackend[EitherT[IO, Throwable, ?], fs2.Stream[IO, ByteBuffer]]

  private val sttpResource: Resource[IO, STTP] =
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

    implicit val b: Backoff[EffectError] = Backoff.default

    MasterConfig
      .load()
      .map(mc ⇒ mc -> LogFactory.forPrintln[IO](mc.logLevel))
      .flatMap {
        case (masterConf, lf) =>
          implicit val logFactory: LogFactory[IO] = lf
          logFactory.init("node", "run") >>= { implicit log: Log[IO] ⇒
            // Run master node and status server
            (for {
              implicit0(sttp: STTP) ← sttpResource
              implicit0(dockerIO: DockerIO[IO]) ← DockerIO.make[IO]()

              conf ← Resource.liftF(Configuration.init[IO](masterConf))
              kad ← kademlia(conf.rootPath, masterConf.kademlia)
              pool ← dockerWorkersPool(conf.rootPath, masterConf)

              node ← MasterNode.make[IO, UriContact](masterConf, conf.nodeConfig, pool, kad.kademlia)
            } yield (kad.http, node)).use {
              case (kadHttp, node) ⇒
                (for {
                  _ ← Log.resource[IO].debug(s"Eth contract config: ${masterConf.contract}")
                  server ← masterHttp(masterConf, node, kadHttp)
                } yield server).use { server =>
                  log.info("Http api server has started on: " + server.address) *> node.run
                }
            }.guaranteeCase {
              case Canceled =>
                log.error("MasterNodeApp was canceled")
              case Error(e) =>
                log.error("MasterNodeApp stopped with error: {}", e)
              case Completed =>
                log.info("MasterNodeApp exited gracefully")
            }
          }
      }
  }

  private def ipfsUploader(conf: RemoteStorageConfig)(implicit sttp: STTP) =
    IpfsUploader[IO](conf.ipfs.address, conf.enabled, conf.ipfs.readTimeout)

  private def dockerWorkersPool(
    rootPath: Path,
    conf: MasterConfig
  )(implicit sttp: STTP, log: Log[IO], dio: DockerIO[IO], backoff: Backoff[EffectError]) =
    DockerWorkersPool.make(
      conf.ports.minPort,
      conf.ports.maxPort,
      rootPath,
      conf.logLevel,
      // TODO: use generic decentralized storage for block uploading instead of IpfsUploader
      BlockUploading(ipfsUploader(conf.remoteStorage))
    )

  private def kademlia(rootPath: Path, conf: KademliaConfig)(implicit sttp: STTP, log: Log[IO]) =
    Resource
      .liftF(Configuration.readTendermintKeyPair(rootPath))
      .flatMap(
        keyPair =>
          KademliaHttpNode.make[IO, IO.Par](
            conf,
            Ed25519.signAlgo,
            keyPair,
            rootPath
        )
      )

  private def masterHttp(masterConf: MasterConfig,
                         node: MasterNode[IO, UriContact],
                         kademliaHttp: KademliaHttp[IO, UriContact])(implicit log: Log[IO], lf: LogFactory[IO]) =
    StatusAggregator
      .make(masterConf, node)
      .flatMap(
        statusAggregator =>
          MasterHttp.make[IO, IO.Par, UriContact](
            "0.0.0.0",
            masterConf.httpApi.port.toShort,
            statusAggregator,
            node.pool,
            kademliaHttp
        )
      )
}
