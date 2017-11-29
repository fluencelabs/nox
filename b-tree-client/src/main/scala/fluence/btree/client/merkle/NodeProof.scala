package fluence.btree.client.merkle

/**
 * Contains all information needed for recalculating checksum of node with substituting child's checksum, for verifying
 * state of the node or recalculating checksum of the node.
 *
 * For leaf ''childrenChecksums'' is sequence with checksums of each 'key-value' pair of this leaf and ''substitutionIdx''
 * is index for inserting checksum of 'key-value' pair.
 * For branch ''childrenChecksums'' is sequence with checksums for each children of this branch and ''substitutionIdx''
 * is index for inserting checksum of child.
 *
 * @param childrenChecksums Checksums of all node children.
 * @param substitutionIdx   Index for substitution
 */
class NodeProof private (val childrenChecksums: Array[Array[Byte]], val substitutionIdx: Int) {
  override def toString: String = {
    "NodeProof(checksums=" + childrenChecksums.map(new String(_)).mkString("[", ",", "]") + "subIdx=" + substitutionIdx
  }
}

object NodeProof {
  def unapply(proof: NodeProof): Option[(Array[Array[Byte]], Int)] =
    Some(proof.childrenChecksums → proof.substitutionIdx)

  /** Create NodeProof with copy of  ''nodeChildrenHash''. */
  def apply(childrenChecksums: Array[Array[Byte]], substitutionIdx: Int): NodeProof = {
    /* We copy array cause this entity will be going outside the tree and it is possible change tree state via this object.
       But this may be changed in the future for performance reason*/
    val arrayCopy = new Array[Array[Byte]](childrenChecksums.length)
    Array.copy(childrenChecksums, 0, arrayCopy, 0, childrenChecksums.length)
    new NodeProof(arrayCopy, substitutionIdx)
  }
}
