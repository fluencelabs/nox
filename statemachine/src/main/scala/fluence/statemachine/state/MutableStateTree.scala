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

package fluence.statemachine.state

import fluence.statemachine._
import fluence.statemachine.tree.{MerkleTreeNode, TreeNode, TreePath}

/**
 * Mutable wrapper for [[TreeNode]].
 *
 * Used as Consensus state only. All changes are made from Consensus thread only: as a synchronous consequence of
 * `DeliverTx` and `Commit` ABCI methods invocation.
 * See [[https://tendermint.readthedocs.io/projects/tools/en/master/abci-spec.html spec]]
 */
class MutableStateTree {
  private var root: TreeNode = TreeNode.emptyNode

  /**
   * Changes the mutable state by assigned the given new `value` to the target `key`.
   *
   * @param key absolute path to the target key
   * @param newValue new value for the target key
   * @return previous value of the target key, if existed
   */
  def putValue(key: TreePath[StoreKey], newValue: StoreValue): Option[StoreValue] = {
    val result = root.getValue(key)
    root = root.addValue(key, newValue)
    result
  }

  /**
   * Changes the mutable state by removing the value of the node corresponding to the target `key`.
   * It also removes the target node completely, in case it is not the root and becomes empty.
   *
   * @param key absolute path to the target key
   * @return previous value of the target key, if existed
   */
  def removeValue(key: TreePath[StoreKey]): Option[StoreValue] = {
    val result = root.getValue(key)
    root = root.removeValue(key)
    result
  }

  /**
   * Merkelizes current state and returns it as merkelized tree.
   *
   * @return the merkelized tree's root
   */
  def merkelize(): MerkleTreeNode = {
    val result = root.merkelize()
    root = result
    result
  }

  /**
   * Provides current root for read-only operations.
   *
   * @return the current root
   */
  def getRoot: TreeNode = root
}
