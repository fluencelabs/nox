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

package fluence.node.workers
import fluence.node.docker.DockerParams
import fluence.node.tendermint.ClusterData

/**
 * Worker container's params
 */
case class WorkerParams(
  clusterData: ClusterData,
  workerPath: String,
  vmCodePath: String,
  masterNodeContainerId: Option[String],
  image: WorkerImage
) {

  override def toString =
    s"(worker ${clusterData.nodeInfo.node_index} with port $rpcPort for ${clusterData.nodeInfo.clusterName})"

  val rpcPort: Short = clusterData.rpcPort

  /**
   * [[fluence.node.docker.DockerIO.run]]'s command for launching a configured worker
   */
  val dockerCommand: DockerParams.Sealed =
    masterNodeContainerId
      .map(_ + ":ro")
      .foldLeft(
        DockerParams
          .daemonRun()
          .option("-e", s"""CODE_DIR=$vmCodePath""")
          .option("-e", s"""WORKER_DIR=$workerPath""")
          .port(clusterData.p2pPort, 26656)
          .port(rpcPort, 26657)
          .port(clusterData.tmPrometheusPort, 26660)
          .port(clusterData.smPrometheusPort, 26661)
      )(
        _.option("--volumes-from", _)
      )
      .option("--name", clusterData.nodeInfo.nodeName)
      .image(image.imageName)
}
