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

package fluence.statemachine.state
import cats.Monad
import cats.data.StateT
import cats.syntax.functor._
import fluence.statemachine.tree.{MerkleTreeNode, TreeNode}

import scala.language.higherKinds

/**
 * Container for states used for processing all Tendermint requests.
 *
 * @param latestCommittedHeight height for the latest committed block, corresponds to the number of commit since genesis
 * @param consensusState state for Consensus requests
 * @param mempoolState state for Mempool requests
 * @param queryState state for Query requests
 */
case class TendermintState(
  latestCommittedHeight: Long,
  consensusState: TreeNode,
  mempoolState: Option[MerkleTreeNode],
  queryState: Option[MerkleTreeNode]
) {

  /**
   * The latest state with all Merkle hashes calculated.
   */
  def latestMerkelized: MerkleTreeNode = mempoolState.getOrElse(TreeNode.emptyMerkelizedNode)

  /**
   * Switches Consensus, Mempool and Query states:
   * - Consensus (real-time) state merkelized.
   * - Mempool state copied from Consensus state.
   * - Query state copied from previous Mempool state.
   */
  def switch(): TendermintState = {
    val merkelized = consensusState.merkelize()
    TendermintState(
      latestCommittedHeight = latestCommittedHeight + 1,
      consensusState = consensusState,
      mempoolState = Some(merkelized),
      queryState = mempoolState
    )
  }

  /**
   * Obtains a copy of current states with provided Consensus state.
   */
  def withConsensusState(newConsensusState: TreeNode): TendermintState =
    TendermintState(latestCommittedHeight, newConsensusState, mempoolState, queryState)
}

object TendermintState {

  /**
   * Initial [[TendermintState]] for newly started State machine.
   */
  def initial: TendermintState = TendermintState(0, TreeNode.emptyNode, None, None)

  /**
   * Transforms [[TreeNode]]-containing State object
   * into [[TendermintState]]-containing one that modifies `consensusState`.
   *
   * @param modifier [[cats.data.State]] to modify `consensusState`
   * @tparam V run result type
   */
  def modifyConsensusState[F[_]: Monad, V](modifier: StateT[F, TreeNode, V]): StateT[F, TendermintState, V] =
    StateT(
      holder =>
        modifier
          .run(holder.consensusState)
          .map { case (state, v) => (holder.withConsensusState(state), v) }
    )
}
