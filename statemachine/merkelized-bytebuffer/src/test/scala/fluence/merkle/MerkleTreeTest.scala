package fluence.merkle

import java.security.MessageDigest

import org.scalatest.{Matchers, WordSpec}
import scodec.bits.ByteVector

class MerkleTreeTest extends WordSpec with Matchers {
  "check size of tree" in {
    // minimum: root and 2 leafs
    val (_, tree) = TestUtils.initBytesTestMerkle(10, 10)
    tree.allNodes.length shouldBe 3

    // use only part of chunk
    /*val (_, tree2) = TestUtils.initBytesTestMerkle(5, 10)
    tree2.allNodes.length shouldBe 3*/

    val (_, tree22) = TestUtils.initBytesTestMerkle(10, 5)
    tree22.allNodes.length shouldBe 3

    // size of mapped with buffer leafs will be size/chunkSize
    // for perfect tree number of leafs must be a power of two
    // height of tree: log2(leafs)
    // number of nodes: 2^(h+1) - 1 or l*2 - 1
    val (_, tree3) = TestUtils.initBytesTestMerkle(256, 2)
    tree3.allNodes.length shouldBe 255
    tree3.treeHeight shouldBe 7

    val (_, tree4) = TestUtils.initBytesTestMerkle(254, 2)
    tree4.allNodes.length shouldBe 255
    tree4.treeHeight shouldBe 7

    val (_, tree5) = TestUtils.initBytesTestMerkle(258, 2)
    tree5.allNodes.length shouldBe 511
    tree5.treeHeight shouldBe 8

    val (_, tree6) = TestUtils.initBytesTestMerkle(252, 2)
    tree6.allNodes.length shouldBe 255
    tree6.treeHeight shouldBe 7
  }

  "check hash calculation" in {

    val size = 900
    val chunkSize = 25
    val (storage, tree) = TestUtils.initBytesTestMerkle(size, chunkSize)

    tree.showTree()

    val leafsCount = tree.leafsCount

    storage.put(1, 1)
    storage.put(2, 4)
    storage.put(3, 7)

    def appendBuffer() = {
      val charsToAppend = leafsCount * chunkSize - size
      storage.bb.array() ++ Array.fill[Byte](charsToAppend)(0)
    }

    val hash1 = tree.recalculateHash()
    appendBuffer() shouldBe hash1

    storage.put(4, 7)
    val hash2 = tree.recalculateHash()
    appendBuffer() shouldBe hash2

    storage.put(7, 4)
    val hash3 = tree.recalculateHash()
    appendBuffer() shouldBe hash3

    (0 until size).foreach(i => storage.put(i, 8))
    val hash4 = tree.recalculateHash()
    appendBuffer() shouldBe hash4

    storage.put(1, 3)
    val hash5 = tree.recalculateHash()
    appendBuffer() shouldBe hash5
    tree.showTree()

    storage.put(size - 1, 9)
    val hash6 = tree.recalculateHash()
    appendBuffer() shouldBe hash6
    tree.showTree()
  }

  "huge tree" in {
    val startTime = System.currentTimeMillis()
    val size = 1 * 1024 * 1024 * 1024
    val digester = MessageDigest.getInstance("SHA-256")
    val (storage, tree) = TestUtils.initBytesTestMerkle(size, 4 * 1024, digester.digest)

    def rndIndex = (math.random() * size).toInt
    def rndByte = (math.random() * Byte.MaxValue).toByte

    println("init: " + (System.currentTimeMillis() - startTime))

    storage.put(7, 4)

    println("finish put: " + (System.currentTimeMillis() - startTime))

    tree.recalculateHash()

    println("finish recalculate: " + (System.currentTimeMillis() - startTime))

    (1 to 100).foreach { _ =>
      val index = rndIndex
      storage.put(index, rndByte)
    }
    println("finish put: " + (System.currentTimeMillis() - startTime))
    tree.recalculateHash()
    println("finish recalculate: " + (System.currentTimeMillis() - startTime))

    (1 to 1000).foreach { _ =>
      val index = rndIndex
      storage.put(index, rndByte)
    }

    println("finish put: " + (System.currentTimeMillis() - startTime))
    tree.recalculateHash()
    println("finish recalculate: " + (System.currentTimeMillis() - startTime))

    (1 to 10000).foreach { _ =>
      val index = rndIndex
      storage.put(index, rndByte)
    }

    println("finish put: " + (System.currentTimeMillis() - startTime))
    val hash = tree.recalculateHash()
    println("finish recalculate: " + (System.currentTimeMillis() - startTime))
    println("hash: " + ByteVector(hash).toHex)
  }
}
