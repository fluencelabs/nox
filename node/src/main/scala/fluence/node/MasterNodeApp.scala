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
import java.io.File
import java.nio.file.StandardCopyOption.REPLACE_EXISTING
import java.nio.file.{Files, Paths}

import cats.effect.{ExitCode, IO, IOApp}
import fluence.ethclient.EthClient
import fluence.node.docker.DockerParams
import fluence.node.eth.{DeployerContract, DeployerContractConfig}
import fluence.node.tendermint.{ClusterData, NodeInfo}
import slogging.MessageFormatter.DefaultPrefixFormatter
import slogging.{LazyLogging, LogLevel, LoggerConfig, PrintLoggerFactory}
import io.circe.syntax._

object MasterNodeApp extends IOApp with LazyLogging {

  /**
   * Launches a single solver connecting to ethereum blockchain with Deployer contract.
   *
   * @param args 1st: Tendermint key location. 2nd: Tendermint p2p host IP. 3rd and 4th: Tendermint p2p port range.
   */
  override def run(args: List[String]): IO[ExitCode] = {
    configureLogging()
    val solverInfoWithConfig = for {
      solverInfo <- NodeConfig.fromArgs(args)
      config <- IO.fromEither(
        pureconfig
          .loadConfig[DeployerContractConfig]
          .left
          .map(fs ⇒ new IllegalArgumentException(fs.toString))
      )
    } yield (solverInfo, config)

    solverInfoWithConfig.attempt.flatMap {
      case Right((solverInfo, config)) =>
        runMasterNode(solverInfo, config)
      case Left(value) =>
        logger.error("Error: {}", value)
        IO.pure(ExitCode.Error)
    }
  }

  /**
   * Run master node with specified solver information and config.
   *
   * @param nodeConfig information about a node willing to run solvers to join Fluence clusters
   * @param config deployer contract settings
   */
  def runMasterNode(nodeConfig: NodeConfig, config: DeployerContractConfig): IO[ExitCode] =
    EthClient
      .makeHttpResource[IO]()
      .use { ethClient ⇒
        for {
          version ← ethClient.clientVersion[IO]()
          _ = logger.info("eth client version {}", version)

          contract = DeployerContract(ethClient, config)

          _ <- contract.addNode[IO](nodeConfig)

          _ <- contract
            .getAllNodeClusters[IO](nodeConfig)
            .evalTap(runSolverWithClusterData)
            .drain // drop the results, so that demand on events is always provided
            .onFinalize(IO(logger.info("subscription finalized")))
            .compile // Compile to a runnable, in terms of effect IO
            .drain // Switch to IO[Unit]

        } yield ExitCode.Success
      }

  private def runSolverWithClusterData(clusterData: ClusterData): IO[Unit] =
    for {
      _ <- IO { logger.info("joining cluster '{}' as node {}", clusterData.clusterName, clusterData.nodeIndex) }
      _ <- initializeTendermintConfigDirectory(clusterData.nodeInfo, clusterData.longTermLocation)

      dockerWorkDir = System.getProperty("user.dir") + "/statemachine/docker"
      vmCodeDir = dockerWorkDir + "/examples/vmcode-" + clusterData.code

      dockerRunCommand = DockerParams
        .daemonRun()
        .port(clusterData.hostP2PPort, 26656)
        .port(clusterData.hostRpcPort, 26657)
        .port(clusterData.tmPrometheusPort, 26660)
        .port(clusterData.smPrometheusPort, 26661)
        .volume(dockerWorkDir + "/solver", "/solver")
        .volume(vmCodeDir, "/vmcode")
        .volume(tendermintHomeDirectory(clusterData.nodeInfo), "/tendermint")
        .option("--name", clusterData.nodeName)
        .image("fluencelabs/solver:latest")

      _ = logger.info("running {}", dockerRunCommand)

      // TODO remove id
      containerId <- IO { scala.sys.process.Process(dockerRunCommand.command, new File(dockerWorkDir)).!! }
      _ = logger.info("launched container {}", containerId)
    } yield ()

  /**
   * Initialized Tendermint config files:
   * - node_info.json initialized from ''nodeInfo''
   * - node_key.json and priv_validator.json initialized from directory located by ''longTermLocation''
   *
   * @param nodeInfo node & cluster information
   * @param longTermLocation local directory with pre-initialized Tendermint public/private keys
   */
  private def initializeTendermintConfigDirectory(
    nodeInfo: NodeInfo,
    longTermLocation: String
  ): IO[Unit] = {
    for {
      longTermConfigPath <- IO { Paths.get(longTermLocation, "config") }

      tendermintConfigDirectoryPath <- IO { Paths.get(tendermintHomeDirectory(nodeInfo), "config") }
      nodeInfoPath <- IO { tendermintConfigDirectoryPath.resolve("node_info.json") }

      _ <- IO { new File(tendermintConfigDirectoryPath.toString).mkdirs() }
      _ <- IO { Files.write(nodeInfoPath, nodeInfo.asJson.spaces2.getBytes) }
      _ <- IO {
        Files.copy(
          longTermConfigPath.resolve("node_key.json"),
          tendermintConfigDirectoryPath.resolve("node_key.json"),
          REPLACE_EXISTING
        )
      }
      _ <- IO {
        Files.copy(
          longTermConfigPath.resolve("priv_validator.json"),
          tendermintConfigDirectoryPath.resolve("priv_validator.json"),
          REPLACE_EXISTING
        )
      }

      _ = logger.info("node info written to {}", nodeInfoPath)
    } yield ()
  }

  private def tendermintHomeDirectory(nodeInfo: NodeInfo): String = {
    val homeDir = System.getProperty("user.home")
    s"$homeDir/.fluence/nodes/${nodeInfo.clusterName}/node${nodeInfo.node_index}"
  }

  private def configureLogging(): Unit = {
    PrintLoggerFactory.formatter = new DefaultPrefixFormatter(false, false, false)
    LoggerConfig.factory = PrintLoggerFactory()
    LoggerConfig.level = LogLevel.INFO
  }
}
