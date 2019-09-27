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

import java.nio.file.Path

import cats.effect.ExitCase.{Canceled, Completed, Error}
import cats.effect._
import cats.syntax.apply._
import cats.syntax.compose._
import cats.syntax.flatMap._
import cats.syntax.profunctor._
import fluence.codec
import fluence.codec.{CodecError, MonadicalEitherArrow, PureCodec}
import fluence.crypto.Crypto.Hasher
import fluence.crypto.{CryptoError, KeyPair}
import fluence.crypto.eddsa.Ed25519
import fluence.crypto.hash.CryptoHashers
import fluence.effects.docker.DockerIO
import fluence.effects.sttp.{SttpEffect, SttpStreamEffect}
import fluence.effects.{Backoff, EffectError}
import fluence.kad.conf.KademliaConfig
import fluence.kad.contact.UriContact
import fluence.kad.http.{KademliaHttp, KademliaHttpNode}
import fluence.kad.protocol.Key
import fluence.log.{Log, LogFactory}
import fluence.node.config.{Configuration, MasterConfig}
import fluence.node.status.StatusAggregator

import scala.language.higherKinds

object MasterNodeApp extends IOApp {

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
              implicit0(sttp: SttpStreamEffect[IO]) ← SttpEffect.streamResource[IO]
              implicit0(dockerIO: DockerIO[IO]) ← DockerIO.make[IO]()

              conf ← Resource.liftF(Configuration.init[IO](masterConf))
              kad ← kademlia(conf.rootPath, masterConf.kademlia)
              node ← MasterNode.make[IO, UriContact](masterConf, conf.nodeConfig, kad.kademlia)
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

  private def kademlia(rootPath: Path, conf: KademliaConfig)(implicit sttp: SttpEffect[IO], log: Log[IO]) =
    Resource
      .liftF(Configuration.readTendermintKeyPair(rootPath))
      .flatMap(
        keyPair =>
          KademliaHttpNode.make[IO](
            conf,
            Ed25519.signAlgo,
            keyPair,
            rootPath, {
              // Lift Crypto errors for PureCodec errors
              val sha256: codec.PureCodec.Func[Array[Byte], Array[Byte]] =
                PureCodec.fromOtherFunc[CryptoError, Array[Byte], Array[Byte]](
                  CryptoHashers.Sha256
                )(err ⇒ CodecError("Crypto error when building Kademlia Key for Node[UriContact]", Some(err)))

              new UriContact.NodeCodec(
                // We have tendermint's node_id in the Fluence Smart Contract now, which is first 20 bytes of sha256 of the public key
                // That's why we derive Kademlia key by sha1 of the node_id
                // sha1( sha256(p2p_key).take(20 bytes) )
                sha256
                  .rmap(_.take(20))
                  .lmap[KeyPair.Public](_.bytes) >>> Key.sha1
              )
            }
          )
      )

  private def masterHttp(
    masterConf: MasterConfig,
    node: MasterNode[IO, UriContact],
    kademliaHttp: KademliaHttp[IO, UriContact]
  )(implicit log: Log[IO], lf: LogFactory[IO]) =
    StatusAggregator
      .make(masterConf, node)
      .flatMap(
        statusAggregator =>
          MasterHttp.make[IO, UriContact](
            "0.0.0.0",
            masterConf.httpApi.port.toShort,
            statusAggregator,
            node.pool,
            kademliaHttp,
            Nil
          )
      )
}
