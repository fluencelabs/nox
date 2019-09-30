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

import fluence.effects.ethclient.data.{Block, Transaction}
import fluence.node.config.MasterConfig
import fluence.worker.WorkerStatus
import io.circe.generic.semiauto._
import io.circe.{Decoder, Encoder, Json, KeyDecoder, KeyEncoder}
import scodec.bits.ByteVector
import io.circe.syntax._

import scala.concurrent.duration._
import scala.language.postfixOps

/**
 * Master node status.
 *
 * @param ip master node external ip address
 * @param uptime working time of master node
 * @param numberOfWorkers number of registered workers
 * @param workers info about workers
 * @param config config file
 */
case class MasterStatus(
  ip: String,
  uptime: Long,
  numberOfWorkers: Int,
  workers: List[(Long, WorkerStatus)],
  config: MasterConfig
)

object MasterStatus {
  private implicit val encodeEthTx: Encoder[Transaction] = deriveEncoder
  private implicit val encodeEthBlock: Encoder[Block] = deriveEncoder
  private implicit val encodeByteVector: Encoder[ByteVector] = Encoder.encodeString.contramap(_.toHex)
  private implicit val encodeFiniteDuration: Encoder[FiniteDuration] = Encoder.encodeLong.contramap(_.toSeconds)

  private implicit val keyEncoderByteVector: KeyEncoder[ByteVector] = KeyEncoder.instance(_.toHex)
  private implicit val encodeStatusTuple: Encoder[(Long, WorkerStatus)] = {
    case (appId: Long, status: WorkerStatus) ⇒
      Json.obj(
        ("appId", Json.fromLong(appId)),
        ("status", status.asJson)
      )
  }

  implicit val encodeMasterState: Encoder[MasterStatus] = deriveEncoder

// Used for tests
  private implicit val decodeEthTx: Decoder[Transaction] = deriveDecoder
  private implicit val decodeEthBlock: Decoder[Block] = deriveDecoder
  private implicit val decodeByteVector: Decoder[ByteVector] =
    Decoder.decodeString.flatMap(
      ByteVector.fromHex(_).fold(Decoder.failedWithMessage[ByteVector]("Not a hex"))(Decoder.const)
    )
  private implicit val decodeFiniteDuration: Decoder[FiniteDuration] = Decoder.decodeLong.map(_ seconds)
  private implicit val keyDecoderByteVector: KeyDecoder[ByteVector] = KeyDecoder.instance(ByteVector.fromHex(_))

  implicit val decodeMasterState: Decoder[MasterStatus] = deriveDecoder
}
