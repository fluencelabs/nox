package fluence.btree

import fluence.btree.client.{ Key, Value }
import fluence.btree.server.core._

package object server {

  type NodeId = Long
  type Hash = Array[Byte]

  type Node = TreeNode[Key]
  type Branch = BranchNode[Key, NodeId]
  type Leaf = LeafNode[Key, Value]

  type NodeAndId = NodeWithId[NodeId, Node]
  type Trail = TreePath[NodeId, Branch]

}
