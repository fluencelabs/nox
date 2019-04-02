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
import fluence.node.eth.state.StorageType.StorageType
import io.circe.{Decoder, Encoder}
import scodec.bits.ByteVector

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

/**
 * Representations of the reference to content in a decentralized storage. Content could be either a file or a directory.
 * Decentralized storage is meant to be content-addressable, i.e., content could be accessed by its hash.
 *
 * @param storageHash Hash of the content
 * @param storageType Decentralized storage used to store that content
 */
case class StorageRef private[eth] (storageHash: ByteVector, storageType: StorageType)

case object StorageRef {
  private[eth] def apply(storageHash: ByteVector, storageType: ByteVector): StorageRef = {
    new StorageRef(storageHash, convertStorageType(storageType))
  }

  private def convertStorageType(storageType: ByteVector): StorageType = {
    storageType.lastOption.map(_.toInt) match {
      case Some(0) => StorageType.Swarm
      case Some(1) => StorageType.Ipfs
      case _ => StorageType.Ipfs
    }
  }
}
