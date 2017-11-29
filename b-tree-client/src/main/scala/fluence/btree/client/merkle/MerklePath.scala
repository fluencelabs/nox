package fluence.btree.client.merkle

/**
 * The Merkle path traversed from root to leaf. Head of path corresponds root of Merkle tree.
 *
 * @param path Ordered sequence of [[NodeProof]] starts with root node ends with leaf.
 */
case class MerklePath(path: Seq[NodeProof]) {

  override def toString: String = {
    "MerklePath(path=" + path.mkString("[", ",", "]")
  }

  /**
   * Adds ''proof'' to the end of the path and return new [[MerklePath]] instance.
   * Doesn't change the original proof: returns a new proof instead.
   *
   * @param proof New proof for adding
   */
  def add(proof: NodeProof): MerklePath =
    copy(path = this.path :+ proof)

}

object MerklePath {
  def empty: MerklePath = new MerklePath(Seq.empty)
}
