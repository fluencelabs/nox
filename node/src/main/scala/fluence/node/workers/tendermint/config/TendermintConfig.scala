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

package fluence.node.workers.tendermint.config
import java.nio.file.{Files, Path, StandardCopyOption}

import cats.effect.Sync
import com.electronwill.nightconfig.core.file.FileConfig
import fluence.node.workers.tendermint.DockerTendermint
import fluence.worker.eth
import fluence.worker.eth.WorkerPeer
import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}
import io.circe.{Decoder, Encoder}

import scala.language.higherKinds

/**
 * Representation of Tendermint config.toml
 * NOTE: external_address, proxy_app, and moniker are
 * missing here because they are generated in [[ConfigTemplate.updateConfigTOML]]
 *
 * @param logLevel Logging level, could be something like "main:info,p2p:error"
 * @param maxInboundPeers  Maximum number of inbound p2p peers
 * @param mempoolSize Maximum number of transactions in mempool. Txs are stored as linked list, so it's not preallocated.
 * @param mempoolCacheSize Maximum number of tx hashes in mempool cache
 * @param commitTimeoutMs Timeout a proposer should wait before committing block. Wait even if 2/3+ votes received. See https://stackoverflow.com/questions/52790981/confusion-about-tendermint-block-creation-interval/52881658#52881658
 * @param skipCommitTimeout If this is true, proposer wouldn't wait for `commitTimeoutMs`, it will commit immediately after receiving 2/3+ vots
 * @param createEmptyBlocks Create empty blocks when there's no txs. See https://github.com/tendermint/tendermint/issues/3307#issuecomment-463520817
 * @param prometheus If true, enable prometheus metrics
 * @param abciPort Port to connect ABCI to. It's the same for all Tendermint instances created by this Node as they are behind Docker network
 */
case class TendermintConfig(
  logLevel: String,
  maxInboundPeers: Int,
  mempoolSize: Int,
  mempoolCacheSize: Int,
  commitTimeoutMs: Long,
  skipCommitTimeout: Boolean,
  createEmptyBlocks: Boolean,
  prometheus: Boolean,
  abciPort: Short,
  corsAllowedOrigins: Seq[String]
) {
  private val mapping: Map[String, String] = Map(
    "log_level" -> logLevel,
    "p2p.max_num_inbound_peers" -> s"$maxInboundPeers",
    "mempool.size" -> s"$mempoolSize",
    "mempool.cache_size" -> s"$mempoolCacheSize",
    "consensus.timeout_commit" -> s"${commitTimeoutMs}ms",
    "consensus.skip_timeout_commit" -> s"$skipCommitTimeout",
    "consensus.create_empty_blocks" -> s"$createEmptyBlocks",
    "instrumentation.prometheus" -> s"$prometheus"
  )

  /**
   * Takes Tendermint config in TOML at `src` and writes updated config to `dst`
   *
   * @param src Path to Tendermint TOML config
   * @param dst Path to save updated Tendermint TOML config
   * @param workerPeerAddress Tendermint p2p peer address, i.e., [[WorkerPeer.peerAddress]]
   * @param workerIndex Index of current peer among all cluster peers, i.e., [[eth.WorkerPeer.index]]
   * @param abciHost Host to connect ABCI to
   * @param appId App id, as defined by smart contract
   */
  def generate[F[_]: Sync](
    src: Path,
    dst: Path,
    workerPeerAddress: String,
    workerIndex: Int,
    abciHost: String,
    appId: Long
  ): F[Unit] = Sync[F].delay {
    val properties = List(
      "proxy_app" -> s"tcp://$abciHost:$abciPort",
      "moniker" -> s"${appId}_$workerIndex",
      "p2p.external_address" -> workerPeerAddress
    ) ++ mapping ++ TendermintConfig.ConfigConstants

    val tmp = Files.copy(src, Files.createTempFile("config", ".toml"), StandardCopyOption.REPLACE_EXISTING)
    val config = FileConfig.of(tmp)
    config.load()

    val updated = properties.foldLeft[FileConfig](config) {
      case (c, (k, v)) => c.set(k, v); c
    }

    import collection.JavaConverters._
    updated.set("rpc.cors_allowed_origins", corsAllowedOrigins.asJava)

    // used for dial_peers
    updated.set("rpc.unsafe", true)

    updated.save()
    updated.close()

    Files.move(tmp, dst, StandardCopyOption.REPLACE_EXISTING)
  }
}

object TendermintConfig {
  // config.toml properties common for all tendermints
  private val ConfigConstants = Map(
    // default is 127.0.0.1, so set to 0.0.0.0 to work inside docker container
    "rpc.laddr" -> s"tcp://0.0.0.0:${DockerTendermint.RpcPort}",
    // 0.32.0 removed default leveldb, so change it to goleveldb
    "db_backend" -> "goleveldb",
    "p2p.addr_book_strict" -> "false",
    // 10MB
    "p2p.send_rate" -> "10240000",
    // 10MB
    "p2p.recv_rate" -> "10240000",
    // Default is 2s
    "consensus.peer_query_maj23_sleep_duration" -> "500ms",
    // Default is 3s
    "timeout_propose" -> "3s"
  )

  implicit val enc: Encoder[TendermintConfig] = deriveEncoder
  implicit val dec: Decoder[TendermintConfig] = deriveDecoder
}
