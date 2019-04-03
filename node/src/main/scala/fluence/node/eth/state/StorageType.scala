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

package fluence.node.eth.state
import io.circe.{Decoder, Encoder}

/**
 * Type of the decentralized storage
 */
object StorageType extends Enumeration {
  type StorageType = Value

  val Swarm, Ipfs = Value

  def toByte(storageType: StorageType): Byte = storageType match {
    case StorageType.Swarm => 0
    case StorageType.Ipfs => 1
  }

  implicit val decoder: Decoder[StorageType] = Decoder.enumDecoder(StorageType)
  implicit val encoder: Encoder[StorageType] = Encoder.enumEncoder(StorageType)
}
