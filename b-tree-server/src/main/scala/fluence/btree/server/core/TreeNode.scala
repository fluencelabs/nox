package fluence.btree.server.core

import fluence.btree.client.merkle.NodeProof

/**
 * The root of tree elements hierarchy.
 */
sealed trait TreeNode[K] {

  /** Stored search keys */
  val keys: Array[K]

  /** Number of keys inside this tree node (optimization) */
  val size: Int

  /** Digest of this node state */
  val checksum: Array[Byte]
}

/**
 * A leaf element of tree, contains stored values. All leaves are located at the same depth (maximum depth) in the tree.
 * Keys and values are in a one to one relationship. (keys.size == values.size == size)
 *
 * @param keys         Search keys
 * @param values       Stored values
 * @param kvChecksums Array of checksums for each key value pair
 * @param size         Number of keys (or values) inside this leaf (optimization)
 * @param checksum    Hash of leaf state
 *
 * @tparam K The type of search key
 * @tparam V The Type of stored value
 */
case class LeafNode[K, V](
    override val keys: Array[K],
    values: Array[V],
    kvChecksums: Array[Array[Byte]],
    override val size: Int,
    override val checksum: Array[Byte]
) extends TreeNode[K]

object LeafNode {

  /** Leaf operations. */
  trait Ops[K, V] {

    /**
     * Updates leaf with new ''key'' and ''value''. Rewrites a key-value pair at the specified index position in the leaf.
     * Old key-value pair will be overwritten with new key-value pair. Index should be between 0 and size of leaf.
     * Doesn't change the original leaf: returns a new leaf instead.
     *
     * @param key   New key for updating
     * @param value New value for updating
     * @param idx   Index for rewriting key and value
     * @return updated leaf with new key and value
     */
    def rewriteKv(key: K, value: V, idx: Int): LeafNode[K, V]

    /**
     * Updates leaf with insert new ''key'' and ''value''. Inserts a key-value pair at the specified index position in the leaf.
     * New key-value pair will be placed at the specified index position, size of leaf will be increased by one.
     * Index should be between 0 and size of leaf. Doesn't change the original leaf: returns a new leaf instead.
     *
     * @param key   New key for inserting
     * @param value New value for inserting
     * @param idx   Index for insertion key and value
     * @return updated leaf with inserted new key and value
     */
    def insertKv(key: K, value: V, idx: Int): LeafNode[K, V]

    /**
     * Splits leaf into two approximately equal parts.
     * Doesn't change the original leaf: returns a two new leaf instead.
     *
     * @return tuple with 2 leafs
     */
    def split: (LeafNode[K, V], LeafNode[K, V])

    /**
     * Returns node proof for current node
     *
     * @param substitutionIdx   Index for substitution childs checksum
     */
    def toProof(substitutionIdx: Int): NodeProof

  }

}

/**
 * Branch node of tree, do not contains values, contains references to child nodes.
 * Сan not be placed at maximum depth of tree.
 * Number of children == number of keys in all branches except last(rightmost) for any tree level.
 * The rightmost branch contain (size + 1) children.
 *
 * @param keys              Search keys
 * @param children         References to children
 * @param childsChecksums Checksum of all children (optimization)
 * @param size              Number of keys inside this branch (optimization)
 * @param checksum         Hash of branch state
 *
 * @tparam K The type of searching Key
 * @tparam C The type of reference to child nodes
 */
case class BranchNode[K, C](
    override val keys: Array[K],
    children: Array[C],
    childsChecksums: Array[Array[Byte]],
    override val size: Int,
    override val checksum: Array[Byte]
) extends TreeNode[K]

object BranchNode {

  /** Branch node operations. */
  trait Ops[K, C] {

    /**
     * Inserts a child at the specified index position in the branch.
     * Doesn't change the original branch: returns a new branch instead.
     *
     * @param key       New key for inserting
     * @param childRef  New child node for inserting
     * @param idx       Index for insertion new child
     * @return Updated branch with inserted new key and child id
     */
    def insertChild(key: K, childRef: ChildRef[C], idx: Int): BranchNode[K, C]

    /**
     * Updates a checksum for the child specified by the index and recalculates the branch checksum.
     * Doesn't change the original branch: returns a new branch instead.
     *
     * @param checksum New checksum of updated child
     * @param idx       Index for doing update
     * @return Updated branch node
     */
    def updateChildChecksum(checksum: Array[Byte], idx: Int): BranchNode[K, C]

    /**
     * Splits leaf into two approximately equal parts.
     *
     * @return tuple with 2 branch node
     */
    def split: (BranchNode[K, C], BranchNode[K, C])

    /**
     * Returns node proof for current node
     *
     * @param substitutionIdx   Index for substitution childs checksum
     */
    def toProof(substitutionIdx: Int): NodeProof

  }

}

/** Wrapper for the node with its corresponding id. */
case class NodeWithId[+Id, +Node <: TreeNode[_]](id: Id, node: Node)

/** Wrapper of the child id and the checksum. */
case class ChildRef[Id](id: Id, checksum: Array[Byte])
