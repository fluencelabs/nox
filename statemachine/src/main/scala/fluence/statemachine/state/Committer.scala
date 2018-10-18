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
import java.text.SimpleDateFormat
import java.util.Calendar

import cats.Monad
import cats.data.StateT
import cats.syntax.functor._
import com.google.protobuf.ByteString
import fluence.statemachine.StoreValue
import fluence.statemachine.tree.{MerkleTreeNode, StoragePaths, TreeNode}
import fluence.statemachine.tx.VmOperationInvoker
import fluence.statemachine.util.HexCodec
import io.prometheus.client.Counter

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

  private val commitDateFormat = new SimpleDateFormat("hh:mm:ss.SSS")
  private val commitCounter: Counter = Counter
    .build()
    .name("solver_commit_count")
    .help("solver_commit_count")
    .register()
  private val commitTimeCounter: Counter = Counter
    .build()
    .name("solver_commit_time_sum")
    .help("solver_commit_time_sum")
    .register()

  /**
   * Handles `Commit` ABCI method (in Consensus thread).
   *
   * @return app hash for Tendermint
   */
  def processCommit(): F[ByteString] = {
    val commitStartTime = System.currentTimeMillis()

    stateHolder.modifyStates(
      for {
        // 2 State monads composed because an atomic update required for Commit
        _ <- TendermintState.modifyConsensusState(preCommitConsensusStateUpdate())
        stateAndHeight <- commitStatesUpdate()

        (state, height) = stateAndHeight
        appHash = ByteString.copyFrom(state.merkleHash.bytes.toArray)
        _ = logState(state, height, commitStartTime)
      } yield appHash
    )
  }

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
          newConsensusState = oldConsensusState.putValue(StoragePaths.VmStateHashPath, vmStateHash)
        } yield (newConsensusState, ())
    )

  /**
   * Switches states and returns the resulting app hash.
   *
   */
  private def commitStatesUpdate(): StateT[F, TendermintState, (MerkleTreeNode, Long)] = StateT(
    oldStates =>
      for {
        newStates <- F.pure(oldStates.switch())
        merkelized = newStates.latestMerkelized
        height = newStates.latestCommittedHeight
      } yield (newStates, (merkelized, height))
  )

  private def logState(state: MerkleTreeNode, height: Long, commitStartTime: Long): Unit = {
    val commitDuration = System.currentTimeMillis() - commitStartTime
    logger.info(
      "{} Commit: processTime={} height={} hash={}",
      commitDateFormat.format(Calendar.getInstance().getTime),
      commitDuration,
      height,
      state.merkleHash.toHex
    )
    logger.debug("\n{}", state.dump())
    logger.info("") // separating messages related to different blocks from each other
    commitCounter.inc()
    commitTimeCounter.inc(commitDuration)
  }

}
