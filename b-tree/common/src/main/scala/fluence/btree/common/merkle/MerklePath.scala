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

package fluence.btree.common.merkle

/**
 * The Merkle path traversed from root to leaf. Head of path corresponds root of Merkle tree.
 *
 * @param path Ordered sequence of [[NodeProof]] starts with root node ends with leaf.
 */
case class MerklePath(path: Seq[NodeProof]) {

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
