package fluence.btree.server.core

import fluence.btree.server.core.TreePath.PathElem
import org.scalatest.{ Matchers, WordSpec }

class TreePathSpec extends WordSpec with Matchers {

  "TreePath" should {
    "addTree and getParent correct" in {

      val branch1 = BranchNode(Array("one"), Array(1), Array(), 1, Array())
      val branch2 = BranchNode(Array("two"), Array(1), Array(), 1, Array())
      val branch3 = BranchNode(Array("three"), Array(1), Array(), 1, Array())

      val ver0 = TreePath.empty[Long, BranchNode[String, Int]]
      val ver1 = ver0.addBranch(1L, branch1, 1)
      val ver2 = ver1.addBranch(2L, branch2, 2)
      val ver3 = ver2.addBranch(3L, branch3, 3)

      ver0.getParent shouldBe None
      ver1.getParent shouldBe Some(PathElem(1L, branch1, 1))
      ver2.getParent shouldBe Some(PathElem(2L, branch2, 2))
      ver3.getParent shouldBe Some(PathElem(3L, branch3, 3))

      ver3.branches should contain theSameElementsInOrderAs Seq(
        PathElem(1L, branch1, 1),
        PathElem(2L, branch2, 2),
        PathElem(3L, branch3, 3)
      )
    }
  }
}
