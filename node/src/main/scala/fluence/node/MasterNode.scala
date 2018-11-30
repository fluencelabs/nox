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
import java.nio.file._

import cats.effect.{ConcurrentEffect, ExitCode, IO, Sync}
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.node.eth.DeployerContract
import fluence.node.solvers.{CodeManager, SolverParams, SolversPool}
import fluence.node.tendermint.{ClusterData, Genesis, KeysPath, NodeInfo}
import fluence.swarm.SwarmClient

import scala.io.Source

/**
 * Represents a MasterNode process. Takes cluster forming events from Ethereum, and spawns new solvers to serve them.
 *
 * @param nodeConfig Tendermint/Fluence master node config
 * @param contract DeployerContract to interact with
 * @param pool Solvers pool to launch solvers in
 * @param rootPath MasterNode's working directory, usually /master
 * @param ce Concurrent effect, used to subscribe to Ethereum events
 */
case class MasterNode(
  nodeConfig: NodeConfig,
  contract: DeployerContract,
  pool: SolversPool[IO],
  codeManager: CodeManager[IO],
  rootPath: Path,
  masterNodeContainerId: String
)(implicit ce: ConcurrentEffect[IO])
    extends slogging.LazyLogging {

  // Writes node info & master keys to tendermint directory
  private val prepareSolverConfigs: fs2.Pipe[IO, ClusterData, SolverParams] =
    _.evalMap {
      case cd @ ClusterData(nodeInfo, _, code) =>
        for {
          _ ← IO { logger.info("joining cluster '{}' as node {}", cd.clusterName, cd.nodeIndex) }

          tmDir ← IO(rootPath.resolve("tendermint"))
          templateConfigDir ← IO(tmDir.resolve("config"))
          solverPath ← IO(tmDir.resolve(nodeInfo.nodeName))
          solverConfigDir ← IO(solverPath.resolve("config"))

          _ ← IO { Files.createDirectories(solverConfigDir) }

          _ ← copyMasterKeys(from = templateConfigDir, to = solverConfigDir)
          _ ← writeNodeInfo(nodeInfo, solverConfigDir)
          _ ← writeGenesis(nodeInfo.cluster.genesis, solverConfigDir)
          _ ← updateConfigTOML(
            nodeInfo,
            configSrc = templateConfigDir.resolve("default_config.toml"),
            configDest = solverConfigDir.resolve("config.toml")
          )

          codePath ← codeManager.prepareCode(code, solverPath)
        } yield SolverParams(cd, solverPath.toString, codePath, masterNodeContainerId)
    }

  private def writeNodeInfo(nodeInfo: NodeInfo, dest: Path) = IO {
    import io.circe.syntax._

    logger.info("Writing {}/node_info.json", dest)
    Files.write(dest.resolve("node_info.json"), nodeInfo.asJson.spaces2.getBytes)
  }

  private def writeGenesis(genesis: Genesis, dest: Path) = IO {
    import io.circe.syntax._

    logger.info("Writing {}/genesis.json", dest)
    Files.write(dest.resolve("genesis.json"), genesis.asJson.spaces2.getBytes)
  }

  private def updateConfigTOML(nodeInfo: NodeInfo, configSrc: Path, configDest: Path) = IO {
    import scala.collection.JavaConverters._
    logger.info("Updating {} -> {}", configSrc, configDest)

    val persistentPeers = nodeInfo.cluster.persistent_peers
    val externalAddress = nodeInfo.cluster.external_addrs(nodeInfo.node_index.toInt)

    val lines = Source.fromFile(configSrc.toUri).getLines().map {
      case s if s.contains("external_address") => s"""external_address = "$externalAddress""""
      case s if s.contains("persistent_peers") => s"""persistent_peers = "$persistentPeers""""
      case s if s.contains("moniker") => s"""moniker = "${nodeInfo.nodeName}""""
      case s => s
    }

    Files.write(configDest, lines.toIterable.asJava)
  }

  private def copyMasterKeys(from: Path, to: Path): IO[Path] = {
    import StandardCopyOption.REPLACE_EXISTING

    val nodeKey = "node_key.json"
    val validator = "priv_validator.json"

    IO {
      logger.info(s"Copying keys to solver: ${from.resolve(nodeKey)} -> ${to.resolve(nodeKey)}")
      Files.copy(from.resolve(nodeKey), to.resolve(nodeKey), REPLACE_EXISTING)

      logger.info(s"Copying priv_validator to solver: ${from.resolve(validator)} -> ${to.resolve(validator)}")
      Files.copy(from.resolve(validator), to.resolve(validator), REPLACE_EXISTING)
    }
  }

  /**
   * Runs MasterNode. Returns when contract.getAllNodeClusters is exhausted
   * TODO: add a way to cleanup, e.g. unsubscribe and stop
   */
  val run: IO[ExitCode] =
    contract
      .getAllNodeClusters(nodeConfig)
      .through(prepareSolverConfigs)
      .evalTap[IO] { params ⇒
        logger.info("Running solver `{}`", params)

        pool.run(params).map(newlyAdded ⇒ logger.info(s"solver run (newly=$newlyAdded) {}", params))
      }
      .drain // drop the results, so that demand on events is always provided
      .onFinalize(IO(logger.info("subscription finalized")))
      .compile // Compile to a runnable, in terms of effect IO
      .drain // Switch to IO[Unit]
      .map(_ ⇒ ExitCode.Success)
}
