package fluence.merkle

import org.scalatest.{Matchers, WordSpec}

class MerkleTreeTest extends WordSpec with Matchers {
  "check size of tree" in {
    val (_, tree) = MerkleTree(10, 10)
    tree.nodes.length shouldBe 1

    // use only part of chunk
    val (_, tree2) = MerkleTree(5, 10)
    tree2.nodes.length shouldBe 1

    val (_, tree22) = MerkleTree(10, 5)
    tree22.nodes.length shouldBe 3

    // size of mapped with buffer leafs will be size/chunkSize
    // for perfect tree number of leafs must be a power of two
    // height of tree: log2(leafs)
    // number of nodes: 2^(h+1) - 1 or l*2 - 1

    val (_, tree3) = MerkleTree(256, 2)
    tree3.nodes.length shouldBe 255
    tree3.treeHeight shouldBe 7

    val (_, tree4) = MerkleTree(255, 2)
    tree4.nodes.length shouldBe 255
    tree4.treeHeight shouldBe 7

    val (_, tree5) = MerkleTree(257, 2)
    tree5.nodes.length shouldBe 511
    tree5.treeHeight shouldBe 8

    val (_, tree6) = MerkleTree(254, 2)
    tree6.nodes.length shouldBe 255
    tree6.treeHeight shouldBe 7
  }
}
