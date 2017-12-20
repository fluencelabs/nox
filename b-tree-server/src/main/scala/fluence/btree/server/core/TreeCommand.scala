package fluence.btree.server.core

import fluence.btree.client.merkle.MerklePath

/**
 * Root interface for all BTree commands.
 *
 * @tparam F The type of effect, box for returning value
 * @tparam K The type of search key
 */
trait TreeCommand[F[_], K] {

  /**
   * Returns next child index to makes next step down the tree.
   * The BTree client searches for required key in the given keys and returns index.
   *
   * @param branch Current branch node of tree
   * @return Index of next child
   */
  def nextChildIndex(branch: BranchNode[K, _]): F[Int]

}

/**
 * Command for searching some value in BTree (by client search key).
 * Search key is stored at the client. BTree server will never know search key.
 *
 * @tparam F The type of effect, box for returning value
 * @tparam K The type of search key
 * @tparam V The type of value
 */
trait GetCommand[F[_], K, V] extends TreeCommand[F, K] {

  /**
   * Sends founded leaf with all keys and values to client.
   * If tree hasn't any leaf sends None.
   *
   * @param leaf Current leaf node of tree
   */
  def submitLeaf(leaf: Option[LeafNode[K, V]]): F[Unit]
}

/**
 * Command for putting key and value to the BTree.
 *
 * @tparam F The type of effect, box for returning value
 * @tparam K The type of search key
 * @tparam V The type of value
 */
trait PutCommand[F[_], K, V] extends TreeCommand[F, K] {

  /**
   * Returns all details needed for putting key and value to BTree.
   *
   * @param leaf Values for calculating current node checksum on the client and find index to insert.
   * @return  Data structure with putting details.
   */
  def putDetails(leaf: Option[LeafNode[K, V]]): F[PutDetails]

  /**
   * Sends merkle path to client after putting key-value pair into the tree.
   *
   * @param merklePath   Tree path traveled in tree from root to leaf
   * @param wasSplitting Indicator of the fact that during putting there was a rebalancing
   */
  def submitMerklePath(merklePath: MerklePath, wasSplitting: Boolean): F[Unit]

}
