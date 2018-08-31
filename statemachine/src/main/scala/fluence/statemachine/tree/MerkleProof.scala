/*
 * Copyright (C) 2018  Fluence Labs Limited
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

package fluence.statemachine.tree

/**
 * Merkle proof provided to the client together with the target node's value as a response to read (`Query`) request.
 *
 * In fact this sequence of proof levels.
 * Each proof level 'explains' how some node's Merkle hash calculated: it is a sequence of digests
 * (see [[MerkleTreeNode.merkleItems()]], that gives this Merkle hash being merge (via [[MerkleHash.merge()]].
 * Every proof level corresponds to some node on path from the state tree root (head proof level)
 * to the target node (tail proof level).
 *
 * @param proof two-dimensional list, outer dimension corresponds to proof level, inner – to separate hashes in level
 */
case class MerkleProof(proof: List[List[MerkleHash]]) {

  /**
   * Prepends a lower (more close to the root) level to the existing proof.
   *
   * Merkle proof is built in reverse order, from the target key to the root, that's why this method is used.
   *
   * @param nextLevel the next level for the proof
   */
  def prepend(nextLevel: List[MerkleHash]): MerkleProof = MerkleProof(nextLevel :: proof)

  /**
   * Provides hex representation of [[MerkleProof]].
   */
  def toHex: String = proof.map(level => level.map(_.toHex).mkString(" ")).mkString(", ")

  override def toString: String = toHex
}

object MerkleProof {

  /**
   * Initialized single-level Merkle proof.
   *
   * @param singleLevel a single, the deepest level of the proof
   */
  def fromSingleLevel(singleLevel: List[MerkleHash]): MerkleProof = MerkleProof(List(singleLevel))
}
