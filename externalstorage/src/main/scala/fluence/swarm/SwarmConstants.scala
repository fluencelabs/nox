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

package fluence.swarm

object SwarmConstants {
  val keyLength = 32
  val updateLookupLength = 4 + 4 + keyLength
  val metaHashLength = keyLength

  // 64 bytes ECDSA size + 1 byte of recovery id
  // https://bitcoin.stackexchange.com/questions/38351/ecdsa-v-r-s-what-is-v
  val signatureLength = 65

  // size of chunk in swarm
  val chunkSize = 4096

  /**
   * updateLookupLength bytes
   * 1 byte flags (multihash bool for now)
   * 32 bytes metaHash
   */
  val updateHeaderLength: Short = (updateLookupLength + 1 + metaHashLength).toShort

  /**
   * Update chunk layout
   * Prefix:
   * 2 bytes updateHeaderLength
   * 2 bytes data length
   */
  val chunkPrefixLength = 2 + 2

  // Minimum size is Header + 1 (minimum data length, enforced)
  val minimumUpdateDataLength = updateHeaderLength + 1

  // data length without metadata
  val maxUpdateDataLength = chunkSize - signatureLength - updateHeaderLength - chunkPrefixLength

  // 8 bytes long Time
  val timestampLength = 8
  val frequencyLength = 8

  // 1 byte (nameLength < 255)
  val nameLengthLength = 1

  // size of Ethereum wallet address
  val addressLength = 20

  /**
   * Resource metadata chunk layout:
   * 4 prefix bytes (chunkPrefixLength). The first two set to zero. The second two indicate the length
   * Timestamp: timestampLength bytes
   * frequency: frequencyLength bytes
   * name length: nameLengthLength bytes
   * name (variable length, can be empty, up to 255 bytes)
   * ownerAddr: addressLength
   */
  val minimumMetadataLength = chunkPrefixLength + timestampLength + frequencyLength + nameLengthLength + addressLength
}
