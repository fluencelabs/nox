/*
 * Copyright (C) 2017  Fluence Labs Limited
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

package fluence.btree.common.merkle

import fluence.btree.core.Hash
import fluence.crypto.hash.{ CryptoHasher, TestCryptoHasher }
import org.scalatest.{ Matchers, WordSpec }

class GeneralNodeProofSpec extends WordSpec with Matchers {

  implicit class Str2Hash(str: String) {
    def toHash: Hash = Hash(str.getBytes)
  }

  private val testHasher = TestCryptoHasher.map(Hash(_))

  "calcChecksum" should {
    "calculate correct checksum" when {
      "stateChecksum is empty, childsChecksums is empty, substitution checksum is None" in {
        val result = GeneralNodeProof(Hash.empty, Array.empty, 1)
          .calcChecksum(testHasher, None)
        result.isEmpty shouldBe true
      }
      "stateChecksum is empty, substitution checksum is None" in {
        val result = GeneralNodeProof(Hash.empty, Array("A".toHash, "B".toHash, "C".toHash), 1)
          .calcChecksum(testHasher, None)
        new String(result.bytes) shouldBe "H<ABC>"
      }
      "stateChecksum is empty, substitution checksum is defined" in {
        val result = GeneralNodeProof(Hash.empty, Array("A".toHash, "B".toHash, "C".toHash), 1)
          .calcChecksum(testHasher, Some("X".toHash))
        new String(result.bytes) shouldBe "H<AXC>"
      }
      "stateChecksum is defined, substitution checksum is None" in {
        val result = GeneralNodeProof("STATE_".toHash, Array("A".toHash, "B".toHash, "C".toHash), 1)
          .calcChecksum(testHasher, None)
        new String(result.bytes) shouldBe "H<STATE_ABC>"
      }
      "stateChecksum is defined, substitution checksum is defined" in {
        val result = GeneralNodeProof("STATE_".toHash, Array("A".toHash, "B".toHash, "C".toHash), 1)
          .calcChecksum(testHasher, Some("X".toHash))
        new String(result.bytes) shouldBe "H<STATE_AXC>"
      }
    }
    "throw exception" when {
      "childrenChecksums is empty" in {
        intercept[ArrayIndexOutOfBoundsException] {
          GeneralNodeProof("STATE_".toHash, Array.empty, 0)
            .calcChecksum(testHasher, Some("X".toHash))
        }
      }
      "substitution index out of bound" in {
        intercept[ArrayIndexOutOfBoundsException] {
          GeneralNodeProof("STATE_".toHash, Array("A".toHash, "B".toHash, "C".toHash), 10)
            .calcChecksum(testHasher, Some("X".toHash))

        }
      }
    }
  }

}
