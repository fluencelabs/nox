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
import fluence.crypto.hash.TestCryptoHasher
import org.scalatest.{ Matchers, WordSpec }

class GeneralNodeProofJvmSpec extends WordSpec with Matchers {

  implicit class Str2Hash(str: String) {
    def toHash: Hash = Hash(str.getBytes)
  }

  private val testHasher = TestCryptoHasher.map(Hash(_))

  "calcChecksum" should {
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
