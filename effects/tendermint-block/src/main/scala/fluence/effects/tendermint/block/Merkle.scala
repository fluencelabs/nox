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

package fluence.effects.tendermint.block

import fluence.crypto.hash.CryptoHashers.Sha256
import scodec.bits.ByteVector

object Merkle {
  private val LeafPrefix: Byte = 0
  private val NodePrefix: Byte = 1

  def simpleHashBV(data: List[ByteVector]): ByteVector = {
    ByteVector(simpleHash(data.map(_.toArray)))
  }

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

  def splitPoint(length: Int): Int = {
    // From doc of Integer.numberOfLeadingZeros:
    //  floor(log_2(x)) = 31 - numberOfLeadingZeros(x)
    val inferiorPower = 31 - Integer.numberOfLeadingZeros(length)

    // Math.pow(2, inferiorPower), but avoiding Doubles
    val result = 1 << inferiorPower

    if (result == length) {
      result >> 1
    } else {
      result
    }
  }

  def leafHash(bs: Array[Byte]): Array[Byte] =
    Sha256.unsafe(LeafPrefix +: bs)

  def nodeHash(left: Array[Byte], right: Array[Byte]): Array[Byte] =
    Sha256.unsafe(NodePrefix +: (left ++ right))
}
