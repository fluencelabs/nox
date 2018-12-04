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
import java.nio.file.{Files, Path, Paths}

import cats.effect.{ContextShift, IO}
import fluence.node.config.{MasterConfig, NodeConfig, StatServerConfig, SwarmConfig}
import fluence.node.eth.DeployerContractConfig
import fluence.node.tendermint.KeysPath
import ConfigOps._
import pureconfig.generic.auto._

case class Configuration(
  rootPath: Path,
  masterKeys: KeysPath,
  nodeConfig: NodeConfig,
  contractConfig: DeployerContractConfig,
  swarm: Option[SwarmConfig],
  statistics: StatServerConfig
)

object Configuration {

  def create()(implicit ec: ContextShift[IO]): IO[(MasterConfig, Configuration)] = {
    for {
      masterConfig <- pureconfig.loadConfig[MasterConfig].toIO
      rootPath <- IO(Paths.get(masterConfig.tendermintPath).toAbsolutePath)
      keysPath <- IO(rootPath.resolve("tendermint"))
      masterKeys <- IO(KeysPath(keysPath.toString))
      _ <- IO(Files.createDirectories(keysPath))
      _ ← masterKeys.init
      solverInfo <- NodeConfig(masterKeys, masterConfig.endpoints)
    } yield
      (
        masterConfig,
        Configuration(
          rootPath,
          masterKeys,
          solverInfo,
          masterConfig.deployer,
          masterConfig.swarm,
          masterConfig.statServer
        )
      )
  }
}
