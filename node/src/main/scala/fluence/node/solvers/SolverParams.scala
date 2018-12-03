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

package fluence.node.solvers
import fluence.node.docker.DockerParams
import fluence.node.tendermint.ClusterData

/**
 * Solver container's params
 */
case class SolverParams(
  clusterData: ClusterData,
  solverPath: String,
  vmCodePath: String,
  masterNodeContainerId: String
) {

  override def toString = s"(solver ${clusterData.nodeInfo.node_index} for ${clusterData.nodeInfo.clusterName})"

  def rpcPort: Short = clusterData.RpcPort

  /**
   * [[fluence.node.docker.DockerIO.run]]'s command for launching a configured solver
   */
  val dockerCommand: DockerParams.Sealed =
    DockerParams
      .daemonRun()
      .option("-e", s"""CODE_DIR=$vmCodePath""")
      .option("-e", s"""SOLVER_DIR=$solverPath""")
      .port(clusterData.P2PPort, 26656)
      .port(rpcPort, 26657)
      .port(clusterData.tmPrometheusPort, 26660)
      .port(clusterData.smPrometheusPort, 26661)
      .option("--volumes-from", masterNodeContainerId + ":ro")
      .option("--name", clusterData.nodeInfo.nodeName)
      .image("fluencelabs/solver:2018-dec-demo")
}
