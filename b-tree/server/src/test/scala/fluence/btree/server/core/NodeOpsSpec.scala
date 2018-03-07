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

package fluence.btree.server.core

import fluence.btree.common.{ Bytes, Hash, Key }
import fluence.btree.server.{ Leaf, NodeId }
import fluence.crypto.hash.{ CryptoHasher, TestCryptoHasher }
import org.scalatest.{ Matchers, WordSpec }

class NodeOpsSpec extends WordSpec with Matchers {

  implicit class Str2Key(str: String) {
    def toKey: Key = Key(str.getBytes)
  }

  private implicit class Str2Hash(str: String) {
    def toHash: Hash = Hash(str.getBytes)
  }

  private val hasher = new CryptoHasher[Array[Byte], Hash] {
    //    private val originHasher = JdkCryptoHash.Sha256
    private val originHasher = TestCryptoHasher
    override def hash(msg: Array[Byte]): Hash = Hash(originHasher.hash(msg))
    override def hash(msg1: Array[Byte], msgN: Array[Byte]*): Hash = Hash(originHasher.hash(msg1, msgN: _*))
  }

  private val nodeOps = NodeOps(hasher)
  import nodeOps._

  private val key1 = "k1".toKey
  private val val1Ref = 1l
  private val val1Checksum = "H<v1>".toHash

  private val key2 = "k2".toKey
  private val val2Ref = 2l
  private val val2Checksum = "H<v2>".toHash

  private val key3 = "k3".toKey
  private val val3Ref = 3l
  private val val3Checksum = "H<v3>".toHash

  private val key4 = "k4".toKey
  private val val4Ref = 4l
  private val val4Checksum = "H<v4>".toHash

  private val key5 = "k5".toKey
  private val val5Ref = 5l
  private val val5Checksum = "H<v5>".toHash

  private val key6 = "k6".toKey
  private val val6Ref = 6l
  private val val6Checksum = "H<v6>".toHash

  private val key7 = "k7".toKey
  private val val7Ref = 7l
  private val val7Checksum = "H<v7>".toHash

  private val kV1Hash = "H<k1H<v1>>".toHash
  private val kV2Hash = "H<k2H<v2>>".toHash
  private val kV3Hash = "H<k3H<v3>>".toHash
  private val kV4Hash = "H<k4H<v4>>".toHash
  private val kV5Hash = "H<k5H<v5>>".toHash
  private val kV6Hash = "H<k6H<v6>>".toHash
  private val kV7Hash = "H<k7H<v7>>".toHash
  private val kV8Hash = "H<k8H<v8>>".toHash

  "LeafOps.rewriteValue" should {
    def keys = Array(key2, key4, key6)
    def valuesRefs = Array(val2Ref, val4Ref, val6Ref)
    def valuesChecksums = Array(val2Checksum, val4Checksum, val6Checksum)

    "rewrite value and return updated leaf" when {
      "rewrite head" in {
        val leaf = newLeaf(keys, valuesRefs, valuesChecksums)
        val updatedLeaf = leaf.rewrite(key1, val1Ref, val1Checksum, 0)

        updatedLeaf.keys should contain theSameElementsInOrderAs Array(key1, key4, key6)
        updatedLeaf.valuesReferences should contain theSameElementsInOrderAs Array(val1Ref, val4Ref, val6Ref)
        updatedLeaf.valuesChecksums should contain theSameElementsInOrderAs Array(val1Checksum, val4Checksum, val6Checksum)
        checkLeafHashes(updatedLeaf, Array(kV1Hash, kV4Hash, kV6Hash), leaf.size)
      }
      "rewrite mid" in {
        val leaf = newLeaf(keys, valuesRefs, valuesChecksums)
        val updatedLeaf = leaf.rewrite(key5, val5Ref, val5Checksum, 1)

        updatedLeaf.keys should contain theSameElementsInOrderAs Array(key2, key5, key6)
        updatedLeaf.valuesReferences should contain theSameElementsInOrderAs Array(val2Ref, val5Ref, val6Ref)
        updatedLeaf.valuesChecksums should contain theSameElementsInOrderAs Array(val2Checksum, val5Checksum, val6Checksum)
        checkLeafHashes(updatedLeaf, Array(kV2Hash, kV5Hash, kV6Hash), leaf.size)
      }
      "rewrite last" in {
        val leaf = newLeaf(keys, valuesRefs, valuesChecksums)
        val updatedLeaf = leaf.rewrite(key7, val7Ref, val7Checksum, 2)

        updatedLeaf.keys should contain theSameElementsInOrderAs Array(key2, key4, key7)
        updatedLeaf.valuesReferences should contain theSameElementsInOrderAs Array(val2Ref, val4Ref, val7Ref)
        updatedLeaf.valuesChecksums should contain theSameElementsInOrderAs Array(val2Checksum, val4Checksum, val7Checksum)
        checkLeafHashes(updatedLeaf, Array(kV2Hash, kV4Hash, kV7Hash), leaf.size)
      }
    }

    "fail when idx out of bounds" in {
      val leaf = newLeaf(keys, valuesRefs, valuesChecksums)
      intercept[Throwable] {
        leaf.rewrite(key7, val7Ref, val7Checksum, -1)
      }
      intercept[Throwable] {
        leaf.rewrite(key7, val7Ref, val7Checksum, 10)
      }
    }

  }

  "LeafOps.insertValue" should {

    def keys = Array(key2, key4, key6)
    def valuesRefs = Array(val2Ref, val4Ref, val6Ref)
    def valuesChecksums = Array(val2Checksum, val4Checksum, val6Checksum)

    "insert value and return updated leaf" when {
      "insertion to head" in {
        val leaf = newLeaf(keys, valuesRefs, valuesChecksums)
        val updatedLeaf = leaf.insert(key1, val1Ref, val1Checksum, 0)

        updatedLeaf.keys should contain theSameElementsInOrderAs Array(key1, key2, key4, key6)
        updatedLeaf.valuesReferences should contain theSameElementsInOrderAs Array(val1Ref, val2Ref, val4Ref, val6Ref)
        updatedLeaf.valuesChecksums should contain theSameElementsInOrderAs Array(val1Checksum, val2Checksum, val4Checksum, val6Checksum)
        checkLeafHashes(updatedLeaf, Array(kV1Hash, kV2Hash, kV4Hash, kV6Hash), leaf.size + 1)
      }
      "insertion to body" in {
        val leaf = newLeaf(keys, valuesRefs, valuesChecksums)
        val updatedLeaf = leaf.insert(key3, val3Ref, val3Checksum, 1)

        updatedLeaf.keys should contain theSameElementsInOrderAs Array(key2, key3, key4, key6)
        updatedLeaf.valuesReferences should contain theSameElementsInOrderAs Array(val2Ref, val3Ref, val4Ref, val6Ref)
        updatedLeaf.valuesChecksums should contain theSameElementsInOrderAs Array(val2Checksum, val3Checksum, val4Checksum, val6Checksum)
        checkLeafHashes(updatedLeaf, Array(kV2Hash, kV3Hash, kV4Hash, kV6Hash), leaf.size + 1)
      }
      "insertion to tail" in {
        val leaf = newLeaf(keys, valuesRefs, valuesChecksums)
        val updatedLeaf = leaf.insert(key7, val7Ref, val7Checksum, 3)

        updatedLeaf.keys should contain theSameElementsInOrderAs Array(key2, key4, key6, key7)
        updatedLeaf.valuesReferences should contain theSameElementsInOrderAs Array(val2Ref, val4Ref, val6Ref, val7Ref)
        updatedLeaf.valuesChecksums should contain theSameElementsInOrderAs Array(val2Checksum, val4Checksum, val6Checksum, val7Checksum)
        checkLeafHashes(updatedLeaf, Array(kV2Hash, kV4Hash, kV6Hash, kV7Hash), leaf.size + 1)
      }
    }

    "fail when idx out of bounds" in {
      val leaf = newLeaf(keys, valuesRefs, valuesChecksums)
      intercept[Throwable] {
        leaf.insert(key7, val7Ref, val7Checksum, -1)
      }
      intercept[Throwable] {
        leaf.insert(key7, val7Ref, val7Checksum, 10)
      }
    }
  }

  "LeafOps.split" should {
    def keys = Array(key1, key2, key3, key4, key5)
    def valuesRefs = Array(val1Ref, val2Ref, val3Ref, val4Ref, val5Ref)
    def valuesChecksums = Array(val1Checksum, val2Checksum, val3Checksum, val4Checksum, val5Checksum)

    "split leaf in half" in {
      val leaf = newLeaf(keys, valuesRefs, valuesChecksums)

      val (left, right) = leaf.split

      left.keys should contain theSameElementsInOrderAs Array(key1, key2)
      left.valuesReferences should contain theSameElementsInOrderAs Array(val1Ref, val2Ref)
      left.valuesChecksums should contain theSameElementsInOrderAs Array(val1Checksum, val2Checksum)
      checkLeafHashes(left, Array(kV1Hash, kV2Hash), leaf.size / 2)

      right.keys should contain theSameElementsInOrderAs Array(key3, key4, key5)
      right.valuesReferences should contain theSameElementsInOrderAs Array(val3Ref, val4Ref, val5Ref)
      right.valuesChecksums should contain theSameElementsInOrderAs Array(val3Checksum, val4Checksum, val5Checksum)
      checkLeafHashes(right, Array(kV3Hash, kV4Hash, kV5Hash), (leaf.size / 2) + 1)

    }
    "fail if leaf size is even" in {
      intercept[Throwable] {
        newLeaf(Array(key1, key2), Array(val1Ref, val2Ref), Array(val1Checksum, val2Checksum)).split
      }
    }
  }

  private val child1Hash = "child1".toHash
  private val child2Hash = "child2".toHash
  private val child3Hash = "child3".toHash
  private val child4Hash = "child4".toHash
  private val child5Hash = "child5".toHash
  private val child6Hash = "child6".toHash
  private val child7Hash = "child7".toHash
  private val child8Hash = "child8".toHash

  "TreeOps.insertChild" should {

    "insert child and return updated tree" when {
      "one key in tree" in {
        val tree = newTree(Array(key2), Array(2L), Array(child2Hash))
        val updatedTree = tree.insertChild(key1, ChildRef(1L, child1Hash), 0)

        updatedTree.keys should contain theSameElementsInOrderAs Array(key1, key2)
        updatedTree.childsReferences should contain theSameElementsInOrderAs Array(1L, 2L)
        checkTreeHashes(updatedTree, Array(child1Hash, child2Hash), tree.size + 1)
      }

      "one key in rightmost tree" in {
        val tree = newTree(Array(key2), Array(2L, 4L), Array(child2Hash, child4Hash))
        val updatedTree = tree.insertChild(key1, ChildRef(1L, child1Hash), 0)

        updatedTree.keys should contain theSameElementsInOrderAs Array(key1, key2)
        updatedTree.childsReferences should contain theSameElementsInOrderAs Array(1L, 2L, 4L)
        checkTreeHashes(updatedTree, Array(child1Hash, child2Hash, child4Hash), tree.size + 1)
      }

      // important test case, guided key is rightmost child of rightmost parent tree
      "one key in rightmost tree, idx=-1" in {
        val tree = newTree(Array(key2), Array(2L, 4L), Array(child2Hash, child4Hash))
        val updatedTree = tree.insertChild(key3, ChildRef(3L, child3Hash), -1)

        updatedTree.keys should contain theSameElementsInOrderAs Array(key2, key3)
        updatedTree.childsReferences should contain theSameElementsInOrderAs Array(2L, 3L, 4L)
        checkTreeHashes(updatedTree, Array(child2Hash, child3Hash, child4Hash), tree.size + 1)
      }

      def keys = Array(key2, key4, key6)
      def children = Array(2L, 4L, 6L, 8L)
      def childrenHashes = Array(child2Hash, child4Hash, child6Hash, child8Hash)

      "many keys in rightmost tree, insertion to head" in {
        val tree = newTree(keys, children, childrenHashes)
        val updatedTree = tree.insertChild(key1, ChildRef(1L, child1Hash), 0)

        updatedTree.keys should contain theSameElementsInOrderAs Array(key1, key2, key4, key6)
        updatedTree.childsReferences should contain theSameElementsInOrderAs Array(1L, 2L, 4L, 6L, 8L)
        checkTreeHashes(updatedTree, child1Hash +: childrenHashes, tree.size + 1)
      }

      "many keys in rightmost tree,insertion to body" in {
        val tree = newTree(keys, children, childrenHashes)
        val updatedTree = tree.insertChild(key3, ChildRef(3L, child3Hash), 1)

        updatedTree.keys should contain theSameElementsInOrderAs Array(key2, key3, key4, key6)
        updatedTree.childsReferences should contain theSameElementsInOrderAs Array(2L, 3L, 4L, 6L, 8L)
        checkTreeHashes(updatedTree, Array(child2Hash, child3Hash, child4Hash, child6Hash, child8Hash), tree.size + 1)
      }

      "many keys in rightmost tree, insertion to tail" in {
        val tree = newTree(keys, children, childrenHashes)
        val updatedTree = tree.insertChild(key5, ChildRef(5L, child5Hash), 2)

        updatedTree.keys should contain theSameElementsInOrderAs Array(key2, key4, key5, key6)
        updatedTree.childsReferences should contain theSameElementsInOrderAs Array(2L, 4L, 5L, 6L, 8L)
        checkTreeHashes(updatedTree, Array(child2Hash, child4Hash, child5Hash, child6Hash, child8Hash), tree.size + 1)
      }
      // important test case, guided key is rightmost child or rightmost parent tree
      "many keys in rightmost tree, idx=-1" in {
        val tree = newTree(keys, children, childrenHashes)
        val updatedTree = tree.insertChild(key7, ChildRef(7L, child7Hash), -1)

        updatedTree.keys should contain theSameElementsInOrderAs Array(key2, key4, key6, key7)
        updatedTree.childsReferences should contain theSameElementsInOrderAs Array(2L, 4L, 6L, 7L, 8L)
        checkTreeHashes(updatedTree, Array(child2Hash, child4Hash, child6Hash, child7Hash, child8Hash), tree.size + 1)
      }
    }

    "fail with assertion error" when {
      "idx=-1 for regular tree" in {
        intercept[Throwable] {
          val tree = newTree(Array(key2), Array(2L), Array(child2Hash))
          tree.insertChild(key1, ChildRef(1L, child1Hash), -1)
        }
      }
    }
  }

  "TreeOps.updateChildHash" should {
    "update child hash" when {
      "tree has one child" in {
        val tree = newTree(Array(key1), Array(1L), Array(child1Hash))
        val updatedTree = tree.updateChildChecksum(child5Hash, 0)
        checkTreeHashes(updatedTree, Array(child5Hash), 1)
      }
      "tree has two child" in {
        val tree = newTree(Array(key1), Array(1L, 2L), Array(child1Hash, child2Hash))
        val updatedTree = tree.updateChildChecksum(child5Hash, 1)
        checkTreeHashes(updatedTree, Array(child1Hash, child5Hash), 1)
      }
      "tree has many children" in {
        val tree = newTree(Array(key1, key2, key3), Array(1L, 2L, 3L, 4L), Array(child1Hash, child2Hash, child3Hash, child4Hash))
        val updatedTree = tree.updateChildChecksum(child5Hash, 2)
        checkTreeHashes(updatedTree, Array(child1Hash, child2Hash, child5Hash, child4Hash), 3)
      }
    }
  }

  "TreeOps.split" should {
    "split in half" when {
      "tree ins't last on this lvl (keys.size == children.size)" in {
        val keys = Array(key1, key2, key3, key4)
        val children = Array(1L, 2L, 3L, 4L)
        val childrenHashes = Array(child1Hash, child2Hash, child3Hash, child4Hash)
        val tree = newTree(keys, children, childrenHashes)

        val (left, right) = tree.split

        left.keys should contain theSameElementsInOrderAs Array(key1, key2)
        left.childsReferences should contain theSameElementsInOrderAs Array(1L, 2L)
        checkTreeHashes(left, Array(child1Hash, child2Hash), 2)

        right.keys should contain theSameElementsInOrderAs Array(key3, key4)
        right.childsReferences should contain theSameElementsInOrderAs Array(3L, 4L)
        checkTreeHashes(right, Array(child3Hash, child4Hash), 2)
      }
      "tree is the last on this lvl (keys.size + 1 == children.size)" in {
        val keys = Array(key1, key2, key3, key4)
        val children = Array(1L, 2L, 3L, 4L, 5L)
        val childrenHashes = Array(child1Hash, child2Hash, child3Hash, child4Hash, child5Hash)
        val tree = newTree(keys, children, childrenHashes)

        val (left, right) = tree.split

        left.keys should contain theSameElementsInOrderAs Array(key1, key2)
        left.childsReferences should contain theSameElementsInOrderAs Array(1L, 2L)
        checkTreeHashes(left, Array(child1Hash, child2Hash), 2)

        right.keys should contain theSameElementsInOrderAs Array(key3, key4)
        right.childsReferences should contain theSameElementsInOrderAs Array(3L, 4L, 5L)
        checkTreeHashes(right, Array(child3Hash, child4Hash, child5Hash), 2)

      }
    }
  }

  "NodeOps.createLeaf" should {
    "just create a valid leaf" in {
      val leaf = createLeaf(key1, val1Ref, val1Checksum)

      leaf.keys should contain theSameElementsInOrderAs Array(key1)
      leaf.valuesReferences should contain theSameElementsInOrderAs Array(val1Ref)
      leaf.valuesChecksums should contain theSameElementsInOrderAs Array(val1Checksum)
      leaf.size shouldBe 1
      leaf.checksum shouldBe getLeafChecksum(getKvChecksums(Array(key1), Array(val1Checksum)))
    }
  }

  "NodeOps.createTRee" should {
    "just create a valid tree" in {
      val tree = createBranch(key1, ChildRef(1L, child1Hash), ChildRef(2L, child2Hash))

      tree.keys should contain theSameElementsInOrderAs Array(key1)
      tree.childsReferences should contain theSameElementsInOrderAs Array(1L, 2L)
      checkTreeHashes(tree, Array(child1Hash, child2Hash), 1)
    }
  }

  private def checkLeafHashes(
    updatedLeaf: Leaf,
    expectedKVHashes: Array[Hash],
    expectedSize: Int
  ) = {
    updatedLeaf.kvChecksums.asStr should contain theSameElementsInOrderAs expectedKVHashes.asStr
    updatedLeaf.checksum.bytes shouldBe getLeafChecksum(expectedKVHashes).bytes
    updatedLeaf.size shouldBe expectedSize
  }

  private def checkTreeHashes(
    updatedTree: BranchNode[Key, NodeId],
    expectedChildrenHashes: Array[Hash],
    expectedSize: Int
  ) = {
    updatedTree.childsChecksums should contain theSameElementsInOrderAs expectedChildrenHashes
    updatedTree.checksum shouldBe getBranchChecksum(updatedTree.keys, expectedChildrenHashes)
    updatedTree.size shouldBe expectedSize
  }

  private def newLeaf(keys: Array[Key], valuesRef: Array[Long], valuesChecksums: Array[Hash]): Leaf = {
    val kvHashes = getKvChecksums(keys, valuesChecksums)
    LeafNode(keys, valuesRef, valuesChecksums, kvHashes, keys.length, getLeafChecksum(kvHashes))
  }

  private def newTree(keys: Array[Key], children: Array[Long], childrenHashes: Array[Hash]): BranchNode[Key, Long] = {
    BranchNode(keys, children, childrenHashes, keys.length, getBranchChecksum(keys, childrenHashes))
  }

  private implicit class Hashes2Strings(hashArr: Array[Hash]) {
    def asStr: Array[String] = hashArr.map(h ⇒ new String(h.bytes))
  }

}
