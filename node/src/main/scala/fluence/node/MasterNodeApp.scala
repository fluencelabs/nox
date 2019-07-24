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

import cats.data.{EitherT, NonEmptyList}
import cats.effect.ExitCase.{Canceled, Completed, Error}
import cats.effect._
import cats.effect.concurrent.{Deferred, Ref}
import cats.syntax.apply._
import cats.syntax.flatMap._
import com.softwaremill.sttp.SttpBackend
import fluence.EitherTSttpBackend
import fluence.crypto.eddsa.Ed25519
import fluence.effects.docker.DockerIO
import fluence.effects.ipfs.IpfsUploader
import fluence.effects.receipt.storage.KVReceiptStorage
import fluence.kad.contact.UriContact
import fluence.kad.http.KademliaHttpNode
import fluence.log.{Log, LogFactory}
import fluence.node.config.{Configuration, MasterConfig}
import fluence.node.status.StatusAggregator
import fluence.node.workers.DockerWorkersPool
import fluence.node.workers.subscription.{RequestResponderImpl, RequestSubscriber, ResponsePromise}
import fluence.node.workers.tendermint.BlockUploading

import scala.language.higherKinds

object MasterNodeApp extends IOApp {
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

    MasterConfig
      .load()
      .map(mc ⇒ mc -> LogFactory.forPrintln[IO](mc.logLevel))
      .flatMap {
        case (masterConf, lf) =>
          implicit val logFactory: LogFactory[IO] = lf
          logFactory.init("node", "run") >>= { implicit log: Log[IO] ⇒
            // Run master node and status server
            (for {
              implicit0(sttp: STTP) <- sttpResource
              implicit0(dockerIO: DockerIO[IO]) <- DockerIO.make[IO]()
              conf <- Resource.liftF(Configuration.init[IO](masterConf))
              // TODO: use generic decentralized storage
              ipfs = IpfsUploader[IO](masterConf.remoteStorage.ipfs.address, masterConf.remoteStorage.enabled)
              blockUploading = BlockUploading.make(ipfs, appId => KVReceiptStorage.make[IO](appId, conf.rootPath))
              ref <- Resource.liftF(
                Ref.of[IO, Map[Long, NonEmptyList[ResponsePromise[IO]]]](
                  Map.empty[Long, NonEmptyList[ResponsePromise[IO]]]
                )
              )
              requestSubscriber = RequestSubscriber(ref)
              pool <- DockerWorkersPool.make(
                masterConf.ports.minPort,
                masterConf.ports.maxPort,
                conf.rootPath,
                blockUploading
              )
              keyPair <- Resource.liftF(Configuration.readTendermintKeyPair(masterConf.rootPath))
              kad ← KademliaHttpNode.make[IO, IO.Par](
                masterConf.kademlia,
                Ed25519.signAlgo,
                keyPair,
                conf.rootPath
              )

              node <- MasterNode.make[IO, UriContact](masterConf, conf.nodeConfig, pool, kad.kademlia)
            } yield (kad.http, node, requestSubscriber)).use {
              case (kadHttp, node, requestSubscriber) ⇒
                (for {
                  _ ← Log.resource[IO].debug(s"eth config ${masterConf.contract}")
                  st ← StatusAggregator.make(masterConf, node)
                  server ← MasterHttp.make[IO, IO.Par, UriContact](
                    "0.0.0.0",
                    masterConf.httpApi.port.toShort,
                    st,
                    node.pool,
                    kadHttp,
                    requestSubscriber
                  )
                } yield server).use { server =>
                  log.info("Http api server has started on: " + server.address) *>
                    node.run
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
}
