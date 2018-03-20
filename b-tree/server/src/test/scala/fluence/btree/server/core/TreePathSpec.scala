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

import fluence.btree.core.Hash
import fluence.btree.server.core.TreePath.PathElem
import org.scalatest.{Matchers, WordSpec}

class TreePathSpec extends WordSpec with Matchers {

  "TreePath" should {
    "addTree and getParent correct" in {

      val branch1 = BranchNode(Array("one"), Array(1), Array(), 1, Hash.empty)
      val branch2 = BranchNode(Array("two"), Array(1), Array(), 1, Hash.empty)
      val branch3 = BranchNode(Array("three"), Array(1), Array(), 1, Hash.empty)

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
