package fluence.btree.server.core

import fluence.btree.common.{ Key, Value }
import fluence.btree.server.NodeId
import fluence.hash.TestCryptoHasher
import org.scalatest.{ Matchers, WordSpec }

class NodeOpsSpec extends WordSpec with Matchers {

  type Bytes = Array[Byte]

  private val nodeOps = NodeOps(TestCryptoHasher)
  import nodeOps._

  private val key1 = "k1".getBytes()
  private val val1 = "v1".getBytes()
  private val key2 = "k2".getBytes()
  private val val2 = "v2".getBytes()
  private val key3 = "k3".getBytes()
  private val val3 = "v3".getBytes()
  private val key4 = "k4".getBytes()
  private val val4 = "v4".getBytes()
  private val key5 = "k5".getBytes()
  private val val5 = "v5".getBytes()
  private val key6 = "k6".getBytes()
  private val val6 = "v6".getBytes()
  private val key7 = "k7".getBytes()
  private val val7 = "v7".getBytes()

  private val kV1Hash = "H<k1v1>".getBytes()
  private val kV2Hash = "H<k2v2>".getBytes()
  private val kV3Hash = "H<k3v3>".getBytes()
  private val kV4Hash = "H<k4v4>".getBytes()
  private val kV5Hash = "H<k5v5>".getBytes()
  private val kV6Hash = "H<k6v6>".getBytes()
  private val kV7Hash = "H<k7v7>".getBytes()
  private val kV8Hash = "H<k8v8>".getBytes()

  "LeafOps.rewriteValue" should {
    def keys = Array(key2, key4, key6)
    def values = Array(val2, val4, val6)

    "rewrite value and return updated leaf" when {
      "rewrite head" in {
        val leaf = newLeaf(keys, values)
        val updatedLeaf = leaf.rewriteKv(key1, val1, 0)

        updatedLeaf.keys should contain theSameElementsInOrderAs Array(key1, key4, key6)
        updatedLeaf.values should contain theSameElementsInOrderAs Array(val1, val4, val6)
        checkLeafHashes(updatedLeaf, Array(kV1Hash, kV4Hash, kV6Hash), leaf.size)
      }
      "rewrite mid" in {
        val leaf = newLeaf(keys, values)
        val updatedLeaf = leaf.rewriteKv(key5, val5, 1)

        updatedLeaf.keys should contain theSameElementsInOrderAs Array(key2, key5, key6)
        updatedLeaf.values should contain theSameElementsInOrderAs Array(val2, val5, val6)
        checkLeafHashes(updatedLeaf, Array(kV2Hash, kV5Hash, kV6Hash), leaf.size)
      }
      "rewrite last" in {
        val leaf = newLeaf(keys, values)
        val updatedLeaf = leaf.rewriteKv(key7, val7, 2)

        updatedLeaf.keys should contain theSameElementsInOrderAs Array(key2, key4, key7)
        updatedLeaf.values should contain theSameElementsInOrderAs Array(val2, val4, val7)
        checkLeafHashes(updatedLeaf, Array(kV2Hash, kV4Hash, kV7Hash), leaf.size)
      }
    }

    "fail when idx out of bounds" in {
      val leaf = newLeaf(keys, values)
      intercept[Throwable] {
        leaf.rewriteKv(key7, val7, -1)
      }
      intercept[Throwable] {
        leaf.rewriteKv(key7, val7, 10)
      }
    }

  }

  "LeafOps.insertValue" should {

    def keys = Array(key2, key4, key6)
    def values = Array(val2, val4, val6)

    "insert value and return updated leaf" when {
      "insertion to head" in {
        val leaf = newLeaf(keys, values)
        val updatedLeaf = leaf.insertKv(key1, val1, 0)

        updatedLeaf.keys should contain theSameElementsInOrderAs Array(key1, key2, key4, key6)
        updatedLeaf.values should contain theSameElementsInOrderAs Array(val1, val2, val4, val6)
        checkLeafHashes(updatedLeaf, Array(kV1Hash, kV2Hash, kV4Hash, kV6Hash), leaf.size + 1)
      }
      "insertion to body" in {
        val leaf = newLeaf(keys, values)
        val updatedLeaf = leaf.insertKv(key3, val3, 1)

        updatedLeaf.keys should contain theSameElementsInOrderAs Array(key2, key3, key4, key6)
        updatedLeaf.values should contain theSameElementsInOrderAs Array(val2, val3, val4, val6)
        checkLeafHashes(updatedLeaf, Array(kV2Hash, kV3Hash, kV4Hash, kV6Hash), leaf.size + 1)
      }
      "insertion to tail" in {
        val leaf = newLeaf(keys, values)
        val updatedLeaf = leaf.insertKv(key7, val7, 3)

        updatedLeaf.keys should contain theSameElementsInOrderAs Array(key2, key4, key6, key7)
        updatedLeaf.values should contain theSameElementsInOrderAs Array(val2, val4, val6, val7)
        checkLeafHashes(updatedLeaf, Array(kV2Hash, kV4Hash, kV6Hash, kV7Hash), leaf.size + 1)
      }
    }

    "fail when idx out of bounds" in {
      val leaf = newLeaf(keys, values)
      intercept[Throwable] {
        leaf.insertKv(key7, val7, -1)
      }
      intercept[Throwable] {
        leaf.insertKv(key7, val7, 10)
      }
    }
  }

  "LeafOps.split" should {
    def keys = Array(key1, key2, key3, key4, key5)
    def values = Array(val1, val2, val3, val4, val5)
    "split leaf in half" in {
      val leaf = newLeaf(keys, values)

      val (left, right) = leaf.split

      left.keys should contain theSameElementsInOrderAs Array(key1, key2)
      left.values should contain theSameElementsInOrderAs Array(val1, val2)
      checkLeafHashes(left, Array(kV1Hash, kV2Hash), leaf.size / 2)

      right.keys should contain theSameElementsInOrderAs Array(key3, key4, key5)
      right.values should contain theSameElementsInOrderAs Array(val3, val4, val5)
      checkLeafHashes(right, Array(kV3Hash, kV4Hash, kV5Hash), (leaf.size / 2) + 1)

    }
    "fail if leaf size is even" in {
      intercept[Throwable] {
        newLeaf(Array(key1, key2), Array(val1, val2)).split
      }
    }
  }

  private val child1Hash = "child1".getBytes
  private val child2Hash = "child2".getBytes
  private val child3Hash = "child3".getBytes
  private val child4Hash = "child4".getBytes
  private val child5Hash = "child5".getBytes
  private val child6Hash = "child6".getBytes
  private val child7Hash = "child7".getBytes
  private val child8Hash = "child8".getBytes

  "TreeOps.insertChild" should {

    "insert child and return updated tree" when {
      "one key in tree" in {
        val tree = newTree(Array(key2), Array(2L), Array(child2Hash))
        val updatedTree = tree.insertChild(key1, ChildRef(1L, child1Hash), 0)

        updatedTree.keys should contain theSameElementsInOrderAs Array(key1, key2)
        updatedTree.children should contain theSameElementsInOrderAs Array(1L, 2L)
        checkTreeHashes(updatedTree, Array(child1Hash, child2Hash), tree.size + 1)
      }

      "one key in rightmost tree" in {
        val tree = newTree(Array(key2), Array(2L, 4L), Array(child2Hash, child4Hash))
        val updatedTree = tree.insertChild(key1, ChildRef(1L, child1Hash), 0)

        updatedTree.keys should contain theSameElementsInOrderAs Array(key1, key2)
        updatedTree.children should contain theSameElementsInOrderAs Array(1L, 2L, 4L)
        checkTreeHashes(updatedTree, Array(child1Hash, child2Hash, child4Hash), tree.size + 1)
      }

      // important test case, guided key is rightmost child of rightmost parent tree
      "one key in rightmost tree, idx=-1" in {
        val tree = newTree(Array(key2), Array(2L, 4L), Array(child2Hash, child4Hash))
        val updatedTree = tree.insertChild(key3, ChildRef(3L, child3Hash), -1)

        updatedTree.keys should contain theSameElementsInOrderAs Array(key2, key3)
        updatedTree.children should contain theSameElementsInOrderAs Array(2L, 3L, 4L)
        checkTreeHashes(updatedTree, Array(child2Hash, child3Hash, child4Hash), tree.size + 1)
      }

      def keys = Array(key2, key4, key6)
      def children = Array(2L, 4L, 6L, 8L)
      def childrenHashes = Array(child2Hash, child4Hash, child6Hash, child8Hash)

      "many keys in rightmost tree, insertion to head" in {
        val tree = newTree(keys, children, childrenHashes)
        val updatedTree = tree.insertChild(key1, ChildRef(1L, child1Hash), 0)

        updatedTree.keys should contain theSameElementsInOrderAs Array(key1, key2, key4, key6)
        updatedTree.children should contain theSameElementsInOrderAs Array(1L, 2L, 4L, 6L, 8L)
        checkTreeHashes(updatedTree, child1Hash +: childrenHashes, tree.size + 1)
      }

      "many keys in rightmost tree,insertion to body" in {
        val tree = newTree(keys, children, childrenHashes)
        val updatedTree = tree.insertChild(key3, ChildRef(3L, child3Hash), 1)

        updatedTree.keys should contain theSameElementsInOrderAs Array(key2, key3, key4, key6)
        updatedTree.children should contain theSameElementsInOrderAs Array(2L, 3L, 4L, 6L, 8L)
        checkTreeHashes(updatedTree, Array(child2Hash, child3Hash, child4Hash, child6Hash, child8Hash), tree.size + 1)
      }

      "many keys in rightmost tree, insertion to tail" in {
        val tree = newTree(keys, children, childrenHashes)
        val updatedTree = tree.insertChild(key5, ChildRef(5L, child5Hash), 2)

        updatedTree.keys should contain theSameElementsInOrderAs Array(key2, key4, key5, key6)
        updatedTree.children should contain theSameElementsInOrderAs Array(2L, 4L, 5L, 6L, 8L)
        checkTreeHashes(updatedTree, Array(child2Hash, child4Hash, child5Hash, child6Hash, child8Hash), tree.size + 1)
      }
      // important test case, guided key is rightmost child or rightmost parent tree
      "many keys in rightmost tree, idx=-1" in {
        val tree = newTree(keys, children, childrenHashes)
        val updatedTree = tree.insertChild(key7, ChildRef(7L, child7Hash), -1)

        updatedTree.keys should contain theSameElementsInOrderAs Array(key2, key4, key6, key7)
        updatedTree.children should contain theSameElementsInOrderAs Array(2L, 4L, 6L, 7L, 8L)
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
        left.children should contain theSameElementsInOrderAs Array(1L, 2L)
        checkTreeHashes(left, Array(child1Hash, child2Hash), 2)

        right.keys should contain theSameElementsInOrderAs Array(key3, key4)
        right.children should contain theSameElementsInOrderAs Array(3L, 4L)
        checkTreeHashes(right, Array(child3Hash, child4Hash), 2)
      }
      "tree is the last on this lvl (keys.size + 1 == children.size)" in {
        val keys = Array(key1, key2, key3, key4)
        val children = Array(1L, 2L, 3L, 4L, 5L)
        val childrenHashes = Array(child1Hash, child2Hash, child3Hash, child4Hash, child5Hash)
        val tree = newTree(keys, children, childrenHashes)

        val (left, right) = tree.split

        left.keys should contain theSameElementsInOrderAs Array(key1, key2)
        left.children should contain theSameElementsInOrderAs Array(1L, 2L)
        checkTreeHashes(left, Array(child1Hash, child2Hash), 2)

        right.keys should contain theSameElementsInOrderAs Array(key3, key4)
        right.children should contain theSameElementsInOrderAs Array(3L, 4L, 5L)
        checkTreeHashes(right, Array(child3Hash, child4Hash, child5Hash), 2)

      }
    }
  }

  "NodeOps.createLeaf" should {
    "just create a valid leaf" in {
      val leaf = createLeaf(key1, val1)

      leaf.keys should contain theSameElementsInOrderAs Array(key1)
      leaf.values should contain theSameElementsInOrderAs Array(val1)
      leaf.size shouldBe 1
      leaf.checksum shouldBe getLeafChecksum(getKvChecksums(Array(key1), Array(val1)))
    }
  }

  "NodeOps.createTRee" should {
    "just create a valid tree" in {
      val tree = createBranch(key1, ChildRef(1L, child1Hash), ChildRef(2L, child2Hash))

      tree.keys should contain theSameElementsInOrderAs Array(key1)
      tree.children should contain theSameElementsInOrderAs Array(1L, 2L)
      checkTreeHashes(tree, Array(child1Hash, child2Hash), 1)
    }
  }

  private def checkLeafHashes(
    updatedLeaf: LeafNode[Key, Value],
    expectedKVHashes: Array[Bytes],
    expectedSize: Int
  ) = {
    updatedLeaf.kvChecksums should contain theSameElementsInOrderAs expectedKVHashes
    updatedLeaf.checksum shouldBe getLeafChecksum(expectedKVHashes)
    updatedLeaf.size shouldBe expectedSize
  }

  private def checkTreeHashes(
    updatedTree: BranchNode[Key, NodeId],
    expectedChildrenHashes: Array[Bytes],
    expectedSize: Int
  ) = {
    updatedTree.childsChecksums should contain theSameElementsInOrderAs expectedChildrenHashes
    updatedTree.checksum shouldBe getBranchChecksum(updatedTree.keys, expectedChildrenHashes)
    updatedTree.size shouldBe expectedSize
  }

  private def newLeaf(keys: Array[Bytes], values: Array[Bytes]): LeafNode[Bytes, Bytes] = {
    val kvHashes = getKvChecksums(keys, values)
    LeafNode(keys, values, kvHashes, keys.length, getLeafChecksum(kvHashes))
  }

  private def newTree(keys: Array[Bytes], children: Array[Long], childrenHashes: Array[Bytes]): BranchNode[Bytes, Long] = {
    BranchNode(keys, children, childrenHashes, keys.length, getBranchChecksum(keys, childrenHashes))
  }

}
