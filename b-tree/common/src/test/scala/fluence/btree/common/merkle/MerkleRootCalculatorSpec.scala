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
import fluence.crypto.DumbCrypto
import cats.syntax.profunctor._
import org.scalatest.{Matchers, WordSpec}

class MerkleRootCalculatorSpec extends WordSpec with Matchers {

  implicit class Str2Hash(str: String) {
    def toHash: Hash = Hash(str.getBytes)
  }

  private val testHasher = DumbCrypto.testHasher.rmap(Hash(_))

  private val hash0 = "0".toHash
  private val hash1 = "1".toHash
  private val hash2 = "2".toHash
  private val hash3 = "3".toHash
  private val hashX = "X".toHash

  private val proof = MerkleRootCalculator(testHasher)

  "calcMerkleRoot without hash" should {
    "calculate valid root" when {
      "merkle path has one element with one hash" in {
        val result = proof.calcMerkleRoot(MerklePath(Array(GeneralNodeProof(hash0, Array(hash1), 0))))

        new String(result.bytes) shouldBe "H<01>"
      }
      "merkle path has one element with many hashes" in {
        val result = proof.calcMerkleRoot(MerklePath(Array(GeneralNodeProof(hash0, Array(hash1, hash2, hash3), 0))))

        new String(result.bytes) shouldBe "H<0123>"
      }
      "merkle path has many elements with many hashes" in {
        val node1 = GeneralNodeProof(hash0, Array(hash1, hash1), 0)
        val node2 = GeneralNodeProof(hash0, Array(hash2, hash2), 1)
        val node3 = GeneralNodeProof(hash0, Array(hash3, hash3, hash3), 1)
        val result = proof.calcMerkleRoot(MerklePath(Array(node3, node2, node1)))

        new String(result.bytes) shouldBe "H<03H<02H<011>>3>"
      }
    }

    "returns empty byte array" when {
      "merkle path is empty" in {
        val result = proof.calcMerkleRoot(MerklePath(Array.empty[GeneralNodeProof]))
        result shouldBe Hash.empty
      }
    }
  }

  "calcMerkleRoot with hash" should {
    "substitutes hash and calculate valid root" when {
      "merkle path has one element with one hash" in {
        val result = proof.calcMerkleRoot(MerklePath(Array(GeneralNodeProof(hash0, Array(hash1), 0))), Some(hashX))

        new String(result.bytes) shouldBe "H<0X>"
      }
      "merkle path has one element with many hashes" in {
        val result =
          proof.calcMerkleRoot(MerklePath(Array(GeneralNodeProof(hash0, Array(hash1, hash2, hash3), 1))), Some(hashX))

        new String(result.bytes) shouldBe "H<01X3>"
      }
      "merkle path has many elements with many hashes" in {
        val node1 = GeneralNodeProof(hash0, Array(hash1, hash1), 0)
        val node2 = GeneralNodeProof(hash0, Array(hash2, hash2), 1)
        val node3 = GeneralNodeProof(hash0, Array(hash3, hash3, hash3), 1)
        val result = proof.calcMerkleRoot(MerklePath(Array(node3, node2, node1)), Some(hashX))

        new String(result.bytes) shouldBe "H<03H<02H<0X1>>3>"
      }
    }
  }
}
