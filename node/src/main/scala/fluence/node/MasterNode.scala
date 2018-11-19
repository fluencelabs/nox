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

package fluence.node
import java.nio.file.Paths

import cats.effect.{ConcurrentEffect, ExitCode, IO}
import fluence.node.eth.DeployerContract
import fluence.node.solvers.{SolverParams, SolversPool}
import fluence.node.tendermint.{ClusterData, KeysPath}

/**
 * Represents a MasterNode process. Takes cluster forming events from Ethereum, and spawns new solvers to serve them.
 *
 * @param masterKeys Tendermint keys
 * @param nodeConfig Tendermint/Fluence master node config
 * @param contract DeployerContract to interact with
 * @param pool Solvers pool to launch solvers in
 * @param ce Concurrent effect, used to subscribe to Ethereum events
 */
case class MasterNode(
  masterKeys: KeysPath,
  nodeConfig: NodeConfig,
  contract: DeployerContract,
  pool: SolversPool[IO]
)(
  implicit ce: ConcurrentEffect[IO]
) extends slogging.LazyLogging {

  // Converts ClusterData into SolverParams which is ready to run
  private val clusterDataToParams: fs2.Pipe[IO, ClusterData, SolverParams] =
    _.evalMap(
      clusterData ⇒
        for {
          _ <- IO { logger.info("joining cluster '{}' as node {}", clusterData.clusterName, clusterData.nodeIndex) }
          solverTendermintHome = {
            // TODO remove this
            val homeDir = System.getProperty("user.home")
            s"$homeDir/.fluence/nodes/${clusterData.nodeInfo.clusterName}/node${clusterData.nodeInfo.node_index}"
          }
          solverTendermintPath <- IO(Paths.get(solverTendermintHome))

          _ <- clusterData.nodeInfo.writeTo(solverTendermintPath)
          _ ← masterKeys.copyKeysToSolver(solverTendermintPath)
        } yield {
          logger.info("node info written to {}", solverTendermintPath)

          // TODO move this machinery somewhere
          val dockerWorkDir = System.getProperty("user.dir") + "/statemachine/docker"
          val vmCodeDir = dockerWorkDir + "/examples/vmcode-" + clusterData.code

          SolverParams(
            clusterData,
            solverTendermintHome,
            vmCodeDir,
            dockerWorkDir + "/solver"
          )
      }
    )

  /**
   * Runs MasterNode. Returns when contract.getAllNodeClusters is exhausted
   * TODO: add a way to cleanup, e.g. unsubscribe and stop
   */
  val run: IO[ExitCode] =
    contract
      .getAllNodeClusters[IO](nodeConfig)
      .through(clusterDataToParams)
      .evalTap[IO] { params ⇒
        logger.info("running solver `{}`", params.dockerCommand.command.mkString(" "))

        pool.run(params).map(newlyAdded ⇒ logger.info(s"solver runned(newly=$newlyAdded) {}", params))
      }
      .drain // drop the results, so that demand on events is always provided
      .onFinalize(IO(logger.info("subscription finalized")))
      .compile // Compile to a runnable, in terms of effect IO
      .drain // Switch to IO[Unit]
      .map(_ ⇒ ExitCode.Success)

}
