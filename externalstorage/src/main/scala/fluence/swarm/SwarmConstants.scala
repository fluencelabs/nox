/*
 * Copyright (C) 2018  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
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
