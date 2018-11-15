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

/**
 * Solver container's params
 *
 * @param rpcPort RPC port to bind to
 */
case class SolverParams(rpcPort: Short) {
  override def toString = s"(solver of rpcPort $rpcPort)"

  /**
   * [[fluence.node.docker.DockerIO.run]]'s command for launching a configured solver
   * TODO: replace with a real solver process
   */
  val dockerCommand: DockerParams.Sealed =
    DockerParams
      .daemonRun()
      .port(rpcPort, 80)
      .option("name", s"nginx-$rpcPort")
      .image("nginx")
}
