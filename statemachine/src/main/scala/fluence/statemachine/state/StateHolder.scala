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

import java.util.concurrent.CopyOnWriteArrayList

import com.google.protobuf.ByteString
import fluence.statemachine.tree.{MerkleTreeNode, TreeNode}

/**
 * Processor for `Commit` ABCI method. Also provides various states to another components.
 *
 * Switches Consensus, Mempool and Query states during `Commit` processing:
 * - Consensus (real-time) state merkelized.
 * - Mempool state copied from Consensus state.
 * - Query state copied from previous Mempool state.
 *
 * TODO: Currently all committed states are stored. It is redundant actually.
 *
 * `Height`-th state means the state which `app hash` is contained in `Height`-th block, so this is the state upon
 * `Commit` of `Height-1`-th block.
 * See [[https://tendermint.readthedocs.io/projects/tools/en/master/abci-spec.html spec.]]
 * So, if `H`-th state (state with height=`H`) requested, then this is state obtained on `H-1`-th `Commit` processing
 * (1-indexed) – this is `H-2`-th item of `storage` list (0-indexed).
 *
 * @param consensusState Consensus state, affected by every newly appended transaction
 */
class StateHolder(consensusState: MutableStateTree) extends slogging.LazyLogging {
  private val storage: CopyOnWriteArrayList[MerkleTreeNode] = new CopyOnWriteArrayList[MerkleTreeNode]()

  /**
   * Returns `height` corresponding to the latest committed and, at the same time, the latest verifiable state.
   *
   * Concurrency note: it is possible that the new state is appended between invoking [[latestCommittedHeight]] and
   * getting corresponding state from [[storage]] in [[mempoolState]] or [[queryState()]].
   * Actually it is not a wrong behaviour: Mempool and Query states should not be synchronized somehow with
   * Consensus state.
   */
  def latestCommittedHeight: Long = storage.size()

  /**
   * Returns state used for `CheckTx` method transaction validation.
   */
  def mempoolState: MerkleTreeNode = {
    val mempoolHeight = latestCommittedHeight + 1
    if (mempoolHeight >= 2)
      state(mempoolHeight)
    else
      TreeNode.emptyMerkelizedNode
  }

  /**
   * Returns `Query` state.
   *
   * @return height and corresponding state
   */
  def queryState: Either[String, (Long, MerkleTreeNode)] = {
    val queryHeight = latestCommittedHeight
    Either.cond(queryHeight >= 2, (queryHeight, state(queryHeight)), "Query state is not ready yet")
  }

  /**
   * Handles `Commit` ABCI method (in Consensus thread).
   *
   * @return app hash for Tendermint
   */
  def processCommit(): ByteString = {
    val committedState = consensusState.merkelize()
    // TODO: implicit race condition exists here – next line immediately updates queryState() result, but it's wrong
    storage.add(committedState)
    val appHash = committedState.merkleHash
    logState(committedState)

    ByteString.copyFrom(appHash.bytes.toArray)
  }

  private def state(height: Long): MerkleTreeNode = storage.get((height - 2).toInt)

  private def logState(state: MerkleTreeNode): Unit = {
    logger.info("Commit: height={} hash={}\n{}", latestCommittedHeight, state.merkleHash.toHex, state.dump())
  }
}
