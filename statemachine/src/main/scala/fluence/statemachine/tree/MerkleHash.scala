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

package fluence.statemachine.tree

import fluence.statemachine.util.{Crypto, HexCodec}
import scodec.bits.ByteVector

/**
 * Merkle hash used as hash of [[MerkleTreeNode]]
 *
 * @param bytes binary representation of the hash
 */
case class MerkleHash(bytes: ByteVector) extends AnyVal {
  def toHex: String = HexCodec.binaryToHex(bytes.toArray)
}

object MerkleHash {

  /**
   * Merges sequence of digests.
   *
   * TODO: [[BinaryBasedDigestMergeRule]] might be vulnerable if merged digest might have arbitrary lengths.
   * We need to check it or change the rule.
   *
   * @param parts merkle hashes that take part in merging
   * @param mergeRule describes how to merge parts
   */
  def merge(parts: Seq[MerkleHash], mergeRule: DigestMergeRule = HexBasedDigestMergeRule): MerkleHash =
    mergeRule match {
      case BinaryBasedDigestMergeRule =>
        val buffer = Array.fill[Byte](parts.map(_.bytes.size).sum.toInt)(0)
        val insertPositions = parts.scanLeft(0L)(_ + _.bytes.size).take(parts.length)
        parts.zip(insertPositions).foreach {
          case (part, pos) => Array.copy(part.bytes.toArray, 0, buffer, pos.toInt, part.bytes.size.toInt)
        }
        Crypto.sha3Digest256(buffer)
      case HexBasedDigestMergeRule => Crypto.sha3Digest256(parts.map(_.toHex).mkString(" ").getBytes)
    }
}

sealed trait DigestMergeRule

/**
 * Merge rule that uses a concatenation of merged hashes as an input for digest function.
 */
case object BinaryBasedDigestMergeRule extends DigestMergeRule

/**
 * Merge rule that uses a space-separated concatenation of hex representation of merged hashes
 * as an input for digest function.
 * TODO: get rid of it
 */
case object HexBasedDigestMergeRule extends DigestMergeRule
