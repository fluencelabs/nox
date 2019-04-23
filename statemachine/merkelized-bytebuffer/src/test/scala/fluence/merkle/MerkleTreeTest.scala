package fluence.merkle

import org.scalatest.{Matchers, WordSpec}

class MerkleTreeTest extends WordSpec with Matchers {
  "check size of tree" in {
    // minimum: root and 2 leafs
    val (_, tree) = TestUtils.initTestMerkle(10, 10)
    tree.nodes.length shouldBe 3

    // use only part of chunk
    val (_, tree2) = TestUtils.initTestMerkle(5, 10)
    tree2.nodes.length shouldBe 3

    val (_, tree22) = TestUtils.initTestMerkle(10, 5)
    tree22.nodes.length shouldBe 3

    // size of mapped with buffer leafs will be size/chunkSize
    // for perfect tree number of leafs must be a power of two
    // height of tree: log2(leafs)
    // number of nodes: 2^(h+1) - 1 or l*2 - 1
    val (_, tree3) = TestUtils.initTestMerkle(256, 2)
    tree3.nodes.length shouldBe 255
    tree3.treeHeight shouldBe 7

    val (_, tree4) = TestUtils.initTestMerkle(255, 2)
    tree4.nodes.length shouldBe 255
    tree4.treeHeight shouldBe 7

    val (_, tree5) = TestUtils.initTestMerkle(257, 2)
    tree5.nodes.length shouldBe 511
    tree5.treeHeight shouldBe 8

    val (_, tree6) = TestUtils.initTestMerkle(254, 2)
    tree6.nodes.length shouldBe 255
    tree6.treeHeight shouldBe 7
  }

  "check hash calculation" in {

    val size = 20
    val (storage, tree) = TestUtils.initTestMerkle(size, 1)
    tree.recalculateAll()

    val leafsCount = tree.leafsCount

    storage.put("1", 1, 2)
    storage.put("2", 4, 5)
    storage.put("3", 7, 8)

    def appendBuffer() = {
      val charsToAppend = leafsCount - storage.sb.length()
      storage.sb.toString + ("0" * charsToAppend)
    }

    tree.showTree()

    val hash1 = tree.recalculateHash()
    tree.showTree()
    appendBuffer() shouldBe hash1

    storage.put("4", 7, 8)
    val hash2 = tree.recalculateHash()
    appendBuffer() shouldBe hash2

    storage.put("7", 14, 15)
    val hash3 = tree.recalculateHash()
    appendBuffer() shouldBe hash3

    (0 until size).foreach(i => storage.put((i + 69).toChar.toString, i, i + 1))
    val hash4 = tree.recalculateHash()
    appendBuffer() shouldBe hash4

    storage.put("1", 0, 1)
    val hash5 = tree.recalculateHash()
    appendBuffer() shouldBe hash5

    storage.put("1", size - 1, size)
    val hash6 = tree.recalculateHash()
    appendBuffer() shouldBe hash6
  }
}
