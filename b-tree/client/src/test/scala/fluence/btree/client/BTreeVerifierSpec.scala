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

package fluence.btree.client

import fluence.btree.common.merkle.{ GeneralNodeProof, MerklePath }
import fluence.btree.common.{ ClientPutDetails, Hash, Key }
import fluence.crypto.hash.{ CryptoHasher, TestCryptoHasher }
import org.scalatest.{ Matchers, WordSpec }

import scala.collection.Searching.{ Found, InsertionPoint }

class BTreeVerifierSpec extends WordSpec with Matchers {

  private implicit class Str2Key(str: String) {
    def toKey: Key = Key(str.getBytes)
  }

  private implicit class Str2Hash(str: String) {
    def toHash: Hash = Hash(str.getBytes)
  }

  private val testHasher = new CryptoHasher[Array[Byte], Hash] {
    override def hash(msg: Array[Byte]): Hash = Hash(TestCryptoHasher.hash(msg))
    override def hash(msg1: Array[Byte], msgN: Array[Byte]*): Hash = Hash(TestCryptoHasher.hash(msg1, msgN: _*))
  }

  private val verifier = BTreeVerifier(testHasher)

  private val key1 = "k1".toKey
  private val key2 = "k2".toKey
  private val key3 = "k3".toKey

  private val val1hash = "v1".toHash
  private val val2hash = "v2".toHash
  private val val3hash = "v3".toHash

  private val child1hash = "h1".toHash
  private val child2hash = "h2".toHash
  private val child3hash = "h3".toHash

  private val keys = Array(key1, key2, key3)
  private val valChecksums = Array(val1hash, val2hash, val3hash)
  private val childsChecksums = Array(child1hash, child2hash, child3hash)

  "checkProof" should {
    "return false" when {
      "if server proof isn't correct (root checking)" in {
        val proof = GeneralNodeProof("H<k1k2_WRONG_KEY>".toHash, childsChecksums, 1)
        verifier.checkProof(proof, "H<H<k1k2k3>h1h2h3>".toHash, MerklePath.empty) shouldBe false
      }
      "if server proof isn't correct (second tree lvl checking)" in {
        val proofFromServer = GeneralNodeProof("H<k1k2k3>".toHash, childsChecksums, 0)
        val clientsProofInMerklePath = GeneralNodeProof("not matter".toHash, Array(child1hash, child2hash, child3hash), 1)
        verifier.checkProof(proofFromServer, "not matter".toHash, MerklePath(Seq(clientsProofInMerklePath))) shouldBe false
      }
    }

    "return true" when {
      "if server proof is correct (root checking)" in {
        val proof = GeneralNodeProof("H<k1k2k3>".toHash, childsChecksums, 1)
        verifier.checkProof(proof, "H<H<k1k2k3>h1h2h3>".toHash, MerklePath.empty) shouldBe true
      }

      "if server proof is correct for branch (second tree lvl checking)" in {
        val proofFromServer = GeneralNodeProof("H<k1k2k3>".toHash, childsChecksums, 0)
        val expectedServerProofChecksum = proofFromServer.calcChecksum(testHasher, None)
        val expectedServerProofIdx = 1
        val clientsProofInMerklePath =
          GeneralNodeProof("not matter".toHash, Array(child1hash, expectedServerProofChecksum, child3hash), expectedServerProofIdx)
        verifier.checkProof(proofFromServer, "not matter".toHash, MerklePath(Seq(clientsProofInMerklePath))) shouldBe true
      }
      "if server proof is correct for leaf (second tree lvl checking)" in {
        val proofFromServer = GeneralNodeProof(Hash.empty, childsChecksums, 0)
        val expectedServerProofChecksum = proofFromServer.calcChecksum(testHasher, None)
        val expectedServerProofIdx = 1
        val clientsProofInMerklePath =
          GeneralNodeProof("not matter".toHash, Array(child1hash, expectedServerProofChecksum, child3hash), expectedServerProofIdx)
        verifier.checkProof(proofFromServer, "not matter".toHash, MerklePath(Seq(clientsProofInMerklePath))) shouldBe true
      }
    }
  }

  "getBranchProof" should {
    "returns proof for branch details" in {
      val result = verifier.getBranchProof(keys, childsChecksums, 0)
      result.stateChecksum.bytes shouldBe "H<k1k2k3>".getBytes
      result.childrenChecksums shouldBe childsChecksums
      result.substitutionIdx shouldBe 0
    }
  }

  "getLeafProof" should {
    "returns proof for branch details" in {
      val result = verifier.getLeafProof(keys, valChecksums)
      result.stateChecksum.bytes shouldBe Array.emptyByteArray
      result.childrenChecksums.map(h ⇒ new String(h.bytes)) shouldBe Array("H<k1v1>", "H<k2v2>", "H<k3v3>")
      result.substitutionIdx shouldBe -1
    }
  }

  "newMerkleRoot" should {
    "return new merkle root for simple put operation" when {

      "putting into empty tree" in {
        val clientMPath = MerklePath.empty
        val putDetails = ClientPutDetails(key1, val1hash, InsertionPoint(0))
        val serverMRoot = "H<H<k1v1>>".toHash

        verifier.newMerkleRoot(clientMPath, putDetails, serverMRoot, wasSplitting = false)
      }
      "insert new value" in {
        val clientMPath = MerklePath(Seq(GeneralNodeProof(Hash.empty, Array("H<k2v2>".toHash), 0)))
        val putDetails = ClientPutDetails(key1, val1hash, InsertionPoint(0))
        val serverMRoot = "H<H<k1v1>H<k2v2>>".toHash

        val result = verifier.newMerkleRoot(clientMPath, putDetails, serverMRoot, wasSplitting = false)
        result.get.bytes shouldBe serverMRoot.bytes
      }
      "rewrite old value with new value" in {
        val clientMerklePath = MerklePath(Seq(GeneralNodeProof(Hash.empty, Array("H<k2v2>".toHash), 0)))
        val putDetails = ClientPutDetails(key2, val3hash, Found(0))
        val serverMRoot = "H<H<k2v3>>".toHash

        val result = verifier.newMerkleRoot(clientMerklePath, putDetails, serverMRoot, wasSplitting = false)
        result.get.bytes shouldBe serverMRoot.bytes
      }

      "rewrite old value with new value in second tree lvl" in {
        val rootChildsChecksums = Array("H<H<k1v1>H<k2v2>>".toHash, "H<H<k4v4>H<k5v5>>".toHash)
        val rootProof = verifier.getBranchProof(Array(key2), rootChildsChecksums, 0)
        val leafProof = GeneralNodeProof(Hash.empty, Array("H<k1v1>".toHash, "H<k2v2>".toHash), 1)
        val clientMerklePath = MerklePath(Seq(rootProof, leafProof))
        val putDetails = ClientPutDetails(key2, val3hash, Found(0))
        val serverMRoot = "H<H<k2>H<H<k1v1>H<k2v3>>H<H<k4v4>H<k5v5>>>".toHash

        val result = verifier.newMerkleRoot(clientMerklePath, putDetails, serverMRoot, wasSplitting = false)
        result.get.bytes shouldBe serverMRoot.bytes
      }

      "insert new value in second tree lvl" in {
        val rootChildsChecksums = Array("H<H<k1v1>H<k2v2>>".toHash, "H<H<k4v4>H<k5v5>>".toHash)
        val rootProof = verifier.getBranchProof(Array(key2), rootChildsChecksums, 1)
        val leafProof = GeneralNodeProof(Hash.empty, Array("H<k4v4>".toHash, "H<k5v5>".toHash), 0)
        val clientMerklePath = MerklePath(Seq(rootProof, leafProof))
        val putDetails = ClientPutDetails(key3, val3hash, InsertionPoint(0))
        val serverMRoot = "H<H<k2>H<H<k1v1>H<k2v2>>H<H<k3v3>H<k4v4>H<k5v5>>>".toHash

        val result = verifier.newMerkleRoot(clientMerklePath, putDetails, serverMRoot, wasSplitting = false)
        result.get.bytes shouldBe serverMRoot.bytes
      }

    }

    "return None for simple put operation" when {
      "server mRoot != client mRoot" in {
        val clientMerklePath = MerklePath(Seq(GeneralNodeProof(Hash.empty, Array("H<k2v2>".toHash), 0)))
        val putDetails = ClientPutDetails(key1, val1hash, InsertionPoint(0))
        val serverMRoot = "wrong_root".toHash

        val result = verifier.newMerkleRoot(clientMerklePath, putDetails, serverMRoot, wasSplitting = false)
        result shouldBe None
      }

    }

    "return new merkle root for put operation with tree rebalancing" when {
      // todo implement and write docs
    }

    "return None for put operation with tree rebalancing" when {
      // todo implement and write docs
    }

  }

}
