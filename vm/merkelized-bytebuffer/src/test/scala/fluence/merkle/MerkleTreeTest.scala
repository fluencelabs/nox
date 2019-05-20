package fluence.merkle

import java.security.MessageDigest

import org.scalatest.{Matchers, WordSpec}

class MerkleTreeTest extends WordSpec with Matchers {
  "check size of tree" in {
    // minimum: root and 2 leafs
    val (_, tree) = TestUtils.initBytesTestMerkle(10, 10)
    tree.nodes.length shouldBe 3

    // use only part of chunk
    /*val (_, tree2) = TestUtils.initBytesTestMerkle(5, 10)
    tree2.allNodes.length shouldBe 3*/

    val (_, tree22) = TestUtils.initBytesTestMerkle(10, 5)
    tree22.nodes.length shouldBe 3

    // size of mapped with buffer leafs will be size/chunkSize
    // for perfect tree number of leafs must be a power of two
    // height of tree: log2(leafs)
    // number of nodes: 2^(h+1) - 1 or l*2 - 1
    val (_, tree3) = TestUtils.initBytesTestMerkle(256, 2)
    tree3.nodes.length shouldBe 255
    tree3.treeHeight shouldBe 7

    val (_, tree4) = TestUtils.initBytesTestMerkle(254, 2)
    tree4.nodes.length shouldBe 255
    tree4.treeHeight shouldBe 7

    val (_, tree5) = TestUtils.initBytesTestMerkle(258, 2)
    tree5.nodes.length shouldBe 511
    tree5.treeHeight shouldBe 8

    val (_, tree6) = TestUtils.initBytesTestMerkle(252, 2)
    tree6.nodes.length shouldBe 255
    tree6.treeHeight shouldBe 7
  }

  "check tree initialization for different sizes" in {
    for { i <- 4 to 120 } yield {
      val chunkSize = 2

      val size = 2 * i
      val (storage, tree) = TestUtils.initBytesTestMerkle(size, chunkSize)

      // get hash after initialization
      val hash1 = tree.getHash

      val bitSet = storage.getDirtyChunks
      // recalculate hashes for all chunks
      bitSet.set(0, bitSet.length())
      val hash2 = tree.recalculateHash()
      hash1 shouldBe hash2
    }

  }

  "check hash calculation" in {
    val chunkSize = 25
    // not 1024 to leave the last leafs empty
    val size = 25 * 1022
    val (storage, tree) = TestUtils.initBytesTestMerkle(size, chunkSize)

    storage.put(1, 1)
    storage.put(2, 4)
    storage.put(3, 7)

    def appendBuffer() = {
      val arr = new Array[Byte](size)
      storage.position(0)
      storage.bb.get(arr)
      arr
    }

    val hash1 = tree.recalculateHash()
    hash1 shouldBe appendBuffer()

    storage.put(4, 7)
    val hash2 = tree.recalculateHash()
    hash2 shouldBe appendBuffer()

    storage.put(chunkSize, 4)
    val hash3 = tree.recalculateHash()
    hash3 shouldBe appendBuffer()

    storage.put(chunkSize - 1, 3)
    val hash31 = tree.recalculateHash()
    hash31 shouldBe appendBuffer()

    storage.put(chunkSize + 1, 2)
    val hash32 = tree.recalculateHash()
    hash32 shouldBe appendBuffer()

    (0 until size).foreach(i => storage.put(i, 8))
    val hash4 = tree.recalculateHash()
    hash4 shouldBe appendBuffer()

    storage.put(1, 3)
    val hash5 = tree.recalculateHash()
    hash5 shouldBe appendBuffer()

    storage.put(size - 1, 9)
    val hash6 = tree.recalculateHash()
    hash6 shouldBe appendBuffer()
  }

  "check that correct chunks become dirty in the edge of chunks" in {
    val chunkSize = 10
    val storage = TrackingMemoryBuffer.allocateDirect(100, chunkSize)
    storage.position(8)

    val arr = Array[Byte](1, 2)

    storage.put(arr)

    // we put bytes on two last positions of the chunk, only first chunk must be dirty
    val firstIndex = storage.getDirtyChunks.nextSetBit(0)
    firstIndex shouldBe 0

    val secondIndex = storage.getDirtyChunks.nextSetBit(1)
    secondIndex shouldBe -1
  }

  "huge tree" in {
    val startTime = System.currentTimeMillis()
    val size = 1 * 1024 * 1024 * 1024
    val digester = MessageDigest.getInstance("SHA-256")
    val (storage, tree) =
      TestUtils.initBytesTestMerkle(
        size,
        4 * 1024,
        direct = true,
        digester = Option(digester)
      )

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
    println("hash: " + hash.mkString(" "))
  }
}
