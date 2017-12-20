package fluence.btree

import fluence.btree.client.{ Key, Value }
import fluence.btree.server.core._
import monix.eval.Task

package object server {

  type NodeId = Long
  type Hash = Array[Byte]

  type Node = TreeNode[Key]
  type Branch = BranchNode[Key, NodeId]
  type Leaf = LeafNode[Key, Value]

  type NodeAndId = NodeWithId[NodeId, Node]
  type Trail = TreePath[NodeId, Branch]

  type Get = GetCommand[Task, Key, Value]
  type Put = PutCommand[Task, Key, Value]

}
