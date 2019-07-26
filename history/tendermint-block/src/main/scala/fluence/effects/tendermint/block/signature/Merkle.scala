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

package fluence.effects.tendermint.block.signature

import fluence.crypto.hash.CryptoHashers.Sha256

/**
 * Port of Tendermint's SimpleMerkleHash
 *
 * Uses different prefixes for leafs and nodes in order to
 * avoid second preimage attack, for details, see https://tools.ietf.org/html/rfc6962#section-2.1
 *
 */
private[block] object Merkle {
  private val LeafPrefix: Byte = 0
  private val NodePrefix: Byte = 1

  /**
   * Calculates merkle root (binary) for a list of array bytes
   *
   * @param data List of array bytes, can be of any size
   * @return Merkle hash
   */
  def simpleHash(data: List[Array[Byte]]): Array[Byte] = {
    data.length match {
      case 0 => Array.empty
      case 1 => leafHash(data.head)
      case n =>
        val (l, r) = data.splitAt(splitPoint(n))
        val left = simpleHash(l)
        val right = simpleHash(r)
        nodeHash(left, right)
    }
  }

  /**
   * Calculates lower-nearest power of two.
   *
   * i.e., for some N such that 2**N < length < 2**M, function returns 2**N
   *
   * @param length Number of leafs in the tree
   * @return Power of two, less than length
   */
  private def splitPoint(length: Int): Int = {
    // From doc of Integer.numberOfLeadingZeros:
    //  floor(log_2(x)) = 31 - numberOfLeadingZeros(x)
    val inferiorPower = 31 - Integer.numberOfLeadingZeros(length)

    // Same as Math.pow(2, inferiorPower), but avoiding Doubles
    val result = 1 << inferiorPower

    if (result == length) {
      // We need lower-nearest, but got equal power of two,
      // so apply square root to get lower power of 2
      result >> 1
    } else {
      result
    }
  }

  private def leafHash(bs: Array[Byte]): Array[Byte] =
    Sha256.unsafe(LeafPrefix +: bs)

  private def nodeHash(left: Array[Byte], right: Array[Byte]): Array[Byte] =
    Sha256.unsafe(NodePrefix +: (left ++ right))
}
