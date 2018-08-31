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
import com.google.protobuf.ByteString
import fluence.statemachine.StoreValue
import fluence.statemachine.tree.{MerkleTreeNode, StorageKeys, TreeNode}
import fluence.statemachine.tx.VmOperationInvoker
import fluence.statemachine.util.HexCodec

import scala.language.higherKinds

/**
 * Processor for `Commit` ABCI requests.
 * Initiates [[TendermintState]] switching during `Commit` processing
 *
 * @param stateHolder [[TendermintStateHolder]] used to obtain Consensus state and switch [[TendermintState]] as the result of commit
 * @param vmInvoker a provider of the current VM state
 */
class Committer[F[_]](
  private[statemachine] val stateHolder: TendermintStateHolder[F],
  private val vmInvoker: VmOperationInvoker[F]
)(implicit F: Monad[F])
    extends slogging.LazyLogging {
  private val WrongVmHashValue: StoreValue = "wrong_vm_hash"

  /**
   * Handles `Commit` ABCI method (in Consensus thread).
   *
   * @return app hash for Tendermint
   */
  def processCommit(): F[ByteString] =
    stateHolder.modifyStates(
      for {
        // 2 State monads composed because an atomic update required for Commit
        _ <- TendermintState.modifyConsensusState(preCommitConsensusStateUpdate())
        result <- commitStatesUpdate()
      } yield result
    )

  /**
   * Modifies Consensus state to prepare it for commit.
   * It includes updating VM state hash.
   *
   */
  private def preCommitConsensusStateUpdate(): StateT[F, TreeNode, Unit] =
    StateT(
      oldConsensusState =>
        for {
          vmStateHash <- vmInvoker.vmStateHash().map(HexCodec.binaryToHex).getOrElse(WrongVmHashValue)
          newConsensusState = oldConsensusState.putValue(StorageKeys.vmStateHashKey, vmStateHash)
        } yield (newConsensusState, ())
    )

  /**
   * Switches states and returns the resulting app hash.
   *
   */
  private def commitStatesUpdate(): StateT[F, TendermintState, ByteString] = StateT(
    oldStates =>
      for {
        newStates <- F.pure(oldStates.switch())
        merkelized = newStates.latestMerkelized
        appHash = merkelized.merkleHash
        _ = logState(merkelized, newStates.latestCommittedHeight)
      } yield (newStates, ByteString.copyFrom(appHash.bytes.toArray))
  )

  private def logState(state: MerkleTreeNode, height: Long): Unit =
    logger.info("Commit: height={} hash={}\n{}", height, state.merkleHash.toHex, state.dump())

}
