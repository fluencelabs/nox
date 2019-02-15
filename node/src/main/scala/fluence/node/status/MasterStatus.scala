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

package fluence.node.status
import java.net.InetAddress

import fluence.ethclient.data.{Block, Transaction}
import fluence.node.config.{MasterConfig, NodeConfig}
import fluence.node.eth.NodeEthState
import fluence.node.eth.state.{Cluster, WorkerPeer}
import fluence.node.workers.status.WorkerStatus
import io.circe.{Decoder, Encoder, KeyDecoder, KeyEncoder}
import io.circe.generic.semiauto._
import scodec.bits.ByteVector

import scala.concurrent.duration._

/**
 * Master node status.
 *
 * @param ip master node ip address
 * @param listOfPorts all available ports to use by code developers
 * @param uptime working time of master node
 * @param numberOfWorkers number of registered workers
 * @param workers info about workers
 * @param config config file
 * @param ethState current NodeEthState
 */
case class MasterStatus(
  ip: String,
  listOfPorts: String,
  uptime: Long,
  nodeConfig: NodeConfig,
  numberOfWorkers: Int,
  workers: List[WorkerStatus],
  config: MasterConfig,
  ethState: NodeEthState
)

object MasterStatus {
  private implicit val encodeEthTx: Encoder[Transaction] = deriveEncoder
  private implicit val encodeEthBlock: Encoder[Block] = deriveEncoder
  private implicit val encodeByteVector: Encoder[ByteVector] = Encoder.encodeString.contramap(_.toHex)
  private implicit val encodeInetAddress: Encoder[InetAddress] = Encoder.encodeString.contramap(_.getHostName)
  private implicit val encodeWorkerPeer: Encoder[WorkerPeer] = deriveEncoder
  private implicit val encodeFiniteDuration: Encoder[FiniteDuration] = Encoder.encodeLong.contramap(_.toSeconds)
  private implicit val encodeCluster: Encoder[Cluster] = deriveEncoder
  private implicit val encodeApp: Encoder[fluence.node.eth.state.App] = deriveEncoder
  private implicit val keyEncoderByteVector: KeyEncoder[ByteVector] = KeyEncoder.instance(_.toHex)
  private implicit val encodeNodeEthState: Encoder[NodeEthState] = deriveEncoder
  implicit val encodeMasterState: Encoder[MasterStatus] = deriveEncoder

// Used for tests
  private implicit val decodeEthTx: Decoder[Transaction] = deriveDecoder
  private implicit val decodeEthBlock: Decoder[Block] = deriveDecoder
  private implicit val decodeByteVector: Decoder[ByteVector] =
    Decoder.decodeString.flatMap(
      ByteVector.fromHex(_).fold(Decoder.failedWithMessage[ByteVector]("Not a hex"))(Decoder.const)
    )
  private implicit val decodeInetAddress: Decoder[InetAddress] = Decoder.decodeString.map(InetAddress.getByName)
  private implicit val decodeWorkerPeer: Decoder[WorkerPeer] = deriveDecoder
  private implicit val decodeFiniteDuration: Decoder[FiniteDuration] = Decoder.decodeLong.map(_ seconds)
  private implicit val decodeCluster: Decoder[Cluster] = deriveDecoder
  private implicit val decodeApp: Decoder[fluence.node.eth.state.App] = deriveDecoder
  private implicit val keyDecoderByteVector: KeyDecoder[ByteVector] = KeyDecoder.instance(ByteVector.fromHex(_))
  private implicit val decodeNodeEthState: Decoder[NodeEthState] = deriveDecoder
  implicit val decodeMasterState: Decoder[MasterStatus] = deriveDecoder
}
