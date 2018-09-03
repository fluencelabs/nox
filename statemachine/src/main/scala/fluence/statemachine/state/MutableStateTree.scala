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
import fluence.statemachine._
import fluence.statemachine.tree.{TreeNode, TreePath}

import scala.language.higherKinds

/**
 * Mutable wrapper for [[TreeNode]].
 *
 * Used as Consensus state only. All changes are made from Consensus thread only: as a synchronous consequence of
 * `DeliverTx` and `Commit` ABCI methods invocation.
 * See [[https://tendermint.readthedocs.io/projects/tools/en/master/abci-spec.html spec]]
 *
 * @param stateHolder [[TendermintStateHolder]] instance to provide the current Consensus state for read and write operations
 */
class MutableStateTree[F[_]](private val stateHolder: TendermintStateHolder[F])(implicit F: Monad[F]) {

  /**
   * Changes the mutable state by assigned the given new `value` to the target `key`.
   *
   * @param key absolute path to the target key
   * @param newValue new value for the target key
   * @return previous value of the target key, if existed
   */
  def putValue(key: TreePath[StoreKey], newValue: StoreValue): F[Option[StoreValue]] =
    stateHolder.modifyConsensusState(StateT(state => F.pure(state.putValue(key, newValue), state.getValue(key))))

  /**
   * Changes the mutable state by removing the value of the node corresponding to the target `key`.
   * It also removes the target node completely, in case it is not the root and becomes empty.
   *
   * @param key absolute path to the target key
   * @return previous value of the target key, if existed
   */
  def removeValue(key: TreePath[StoreKey]): F[Option[StoreValue]] =
    stateHolder.modifyConsensusState(StateT(state => F.pure(state.removeValue(key), state.getValue(key))))

  /**
   * Provides current root for read-only operations.
   *
   * @return the current root
   */
  def getRoot: F[TreeNode] = stateHolder.consensusState
}
