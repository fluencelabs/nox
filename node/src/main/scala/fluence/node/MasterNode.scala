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

import cats.effect.{ConcurrentEffect, ExitCode, IO}
import fluence.node.eth.DeployerContract
import fluence.node.solvers.{SolverParams, SolversPool}
import fluence.node.tendermint.{ClusterData, KeysPath, LocalPath, SwarmPath}
import fluence.swarm.SwarmClient

/**
 * Represents a MasterNode process. Takes cluster forming events from Ethereum, and spawns new solvers to serve them.
 *
 * @param masterKeys Tendermint keys
 * @param nodeConfig Tendermint/Fluence master node config
 * @param contract DeployerContract to interact with
 * @param pool Solvers pool to launch solvers in
 * @param path Path to store all the MasterNode's data in
 * @param ce Concurrent effect, used to subscribe to Ethereum events
 */
case class MasterNode(
  masterKeys: KeysPath,
  nodeConfig: NodeConfig,
  contract: DeployerContract,
  pool: SolversPool[IO],
  path: Path
)(
  implicit ce: ConcurrentEffect[IO]
) extends slogging.LazyLogging {

  private def downloadFromSwarmToFile(swarmPath: SwarmPath, filePath: Path): IO[Unit] = {
    SwarmClient(swarmPath.url.getHost, swarmPath.url.getPort).download(swarmPath.path).value.flatMap {
      case Left(err) => IO.raiseError(err)
      case Right(codeBytes) => IO(Files.write(filePath, codeBytes))
    }
  }

  private def downloadAndWriteCodeToFile(solverTendermintPath: Path, swarmPath: SwarmPath): IO[String] =
    for {
      dirPath <- IO(solverTendermintPath.resolve("vmcode-" + swarmPath.path))
      _ <- if (dirPath.toFile.exists()) IO.unit else IO(Files.createDirectory(dirPath))
      filePath <- IO(dirPath.resolve(swarmPath.path + ".wasm"))
      _ <- if (filePath.toFile.exists()) IO.unit
      else
        IO(Files.createFile(filePath))
          .flatMap(_ => downloadFromSwarmToFile(swarmPath, filePath))
    } yield dirPath.toAbsolutePath.toString

  // Converts ClusterData into SolverParams which is ready to run
  private val clusterDataToParams: fs2.Pipe[IO, (ClusterData, Path), SolverParams] =
    _.evalMap {
      case (clusterData, solverTendermintPath) ⇒
        for {
          path <- clusterData.code match {
            case LocalPath(localPath) =>
              IO.pure(Paths.get("./statemachine/docker/examples/vmcode-" + localPath).toAbsolutePath.toString)
            case sp @ SwarmPath(swarmPath, url) =>
              // TODO: test it in integration spec with CLI
              downloadAndWriteCodeToFile(solverTendermintPath, sp)
          }
        } yield {
          SolverParams(
            clusterData,
            solverTendermintPath.toString,
            path
          )
        }
    }

  // Writes node info & master keys to tendermint directory
  private val writeConfigAndKeys: fs2.Pipe[IO, ClusterData, (ClusterData, Path)] =
    _.evalMap(
      clusterData =>
        for {
          _ ← IO { logger.info("joining cluster '{}' as node {}", clusterData.clusterName, clusterData.nodeIndex) }

          solverTendermintPath ← IO(
            path.resolve(s"${clusterData.nodeInfo.clusterName}-${clusterData.nodeInfo.node_index}")
          )

          _ ← clusterData.nodeInfo.writeTo(solverTendermintPath)
          _ ← masterKeys.copyKeysToSolver(solverTendermintPath)
        } yield {
          logger.info("node info written to {}", solverTendermintPath)
          clusterData -> solverTendermintPath
      }
    )

  /**
   * Runs MasterNode. Returns when contract.getAllNodeClusters is exhausted
   * TODO: add a way to cleanup, e.g. unsubscribe and stop
   */
  val run: IO[ExitCode] =
    contract
      .getAllNodeClusters[IO](nodeConfig)
      .through(writeConfigAndKeys)
      .through(clusterDataToParams)
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
