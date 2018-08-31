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
    TendermintState(latestCommittedHeight + 1, consensusState, Some(merkelized), mempoolState)
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
