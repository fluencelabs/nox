/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
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
  def toHex: String = proof.map(level => level.map(_.value.toHex).mkString(" ")).mkString(", ")

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
