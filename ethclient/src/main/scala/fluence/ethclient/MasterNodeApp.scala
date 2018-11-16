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

package fluence.ethclient

import java.io.File
import java.nio.file.StandardCopyOption._
import java.nio.file._

import cats.data.EitherT
import cats.effect.{ExitCode, IO, IOApp}
import cats.implicits._
import fluence.ethclient.Deployer.{CLUSTERFORMED_EVENT, ClusterFormedEventResponse}
import fluence.ethclient.data._
import fluence.ethclient.helpers.DockerRunBuilder
import fluence.ethclient.helpers.JavaRxToFs2._
import fluence.ethclient.helpers.RemoteCallOps._
import fluence.ethclient.helpers.Web3jConverters._
import io.circe.generic.auto._
import io.circe.syntax._
import org.web3j.abi.EventEncoder
import org.web3j.abi.datatypes.generated.Bytes32
import org.web3j.protocol.core.methods.request.EthFilter
import org.web3j.protocol.core.{DefaultBlockParameter, DefaultBlockParameterName}
import slogging.MessageFormatter.DefaultPrefixFormatter
import slogging.{LazyLogging, LogLevel, LoggerConfig, PrintLoggerFactory}

import scala.collection.JavaConverters._
import scala.language.higherKinds
import scala.sys.process._

object MasterNodeApp extends IOApp with LazyLogging {

  /**
   * Launches a single solver connecting to ethereum blockchain with Deployer contract.
   *
   * @param args 1st: Tendermint key location. 2nd: Tendermint p2p host IP. 3rd and 4th: Tendermint p2p port range.
   */
  override def run(args: List[String]): IO[ExitCode] = {
    configureLogging()
    val solverInfoWithConfig = for {
      solverInfo <- SolverInfo(args)
      config <- EitherT
        .fromEither[IO](pureconfig.loadConfig[DeployerContractConfig])
        .leftMap[Throwable](x => new IllegalArgumentException(x.toString))
    } yield (solverInfo, config)

    solverInfoWithConfig.value.flatMap {
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
   * @param solverInfo information about a node willing to run solvers to join Fluence clusters
   * @param config deployer contract settings
   */
  def runMasterNode(solverInfo: SolverInfo, config: DeployerContractConfig): IO[ExitCode] =
    EthClient
      .makeHttpResource[IO]()
      .use { ethClient ⇒
        for {
          version ← ethClient.clientVersion[IO]()
          _ = logger.info("eth client version {}", version)

          contract <- ethClient.getDeployer[IO](config.deployerContractAddress, config.deployerContractOwnerAccount)

          currentBlock = ethClient.web3.ethBlockNumber().send().getBlockNumber
          filter = new EthFilter(
            DefaultBlockParameter.valueOf(currentBlock),
            DefaultBlockParameterName.LATEST,
            config.deployerContractAddress
          ).addSingleTopic(EventEncoder.encode(CLUSTERFORMED_EVENT))

          clusters <- contract.getNodeClusters(solverInfo.validatorKeyBytes32).call[IO]

          _ <- clusters.getValue.asScala.toList
            .map(
              x =>
                contract
                  .getCluster(x)
                  .call[IO]
                  .flatMap(y => processClusterFormed(clusterTupleToClusterFormed(x, y), solverInfo))
            )
            .sequence_

          _ <- contract
            .addNode(
              solverInfo.validatorKeyBytes32,
              solverInfo.addressBytes24,
              solverInfo.startPortUint16,
              solverInfo.endPortUint16
            )
            .call[IO]

          _ <- contract
            .clusterFormedEventObservable(filter)
            .toFS2[IO]
            .evalTap[IO](x ⇒ processClusterFormed(x, solverInfo).map(_ => ()))
            .drain // drop the results, so that demand on events is always provided
            .onFinalize(IO(logger.info("subscription finalized")))
            .compile // Compile to a runnable, in terms of effect IO
            .drain // Switch to IO[Unit]
        } yield ExitCode.Success
      }

  /**
   * Tries to convert event information to cluster configuration and, in case of success, launches a single solver.
   *
   * @param event event from Ethereum Deployer contract
   * @param solverInfo information used to identify the current solver
   * @return whether this event is relevant to the passed SolverInfo
   */
  private def processClusterFormed(
    event: ClusterFormedEventResponse,
    solverInfo: SolverInfo
  ): IO[Boolean] =
    clusterFormedEventToClusterData(event, solverInfo) match {
      case Some(clusterData) => runSolverWithClusterData(clusterData).map(_ => true)
      case None => IO.pure(false)
    }

  private def runSolverWithClusterData(clusterData: ClusterData): IO[Unit] =
    for {
      _ <- IO { logger.info("joining cluster '{}' as node {}", clusterData.clusterName, clusterData.nodeIndex) }
      _ <- initializeTendermintConfigDirectory(clusterData.nodeInfo, clusterData.longTermLocation)

      dockerWorkDir = System.getProperty("user.dir") + "/statemachine/docker"
      vmCodeDir = dockerWorkDir + "/examples/vmcode-" + clusterData.code

      dockerRunCommand = DockerRunBuilder()
        .add("-idt")
        .addPort(clusterData.hostP2PPort, 26656)
        .addPort(clusterData.hostRpcPort, 26657)
        .addPort(clusterData.tmPrometheusPort, 26660)
        .addPort(clusterData.smPrometheusPort, 26661)
        .addVolume(dockerWorkDir + "/solver", "/solver")
        .addVolume(vmCodeDir, "/vmcode")
        .addVolume(tendermintHomeDirectory(clusterData.nodeInfo), "/tendermint")
        .add("--name", clusterData.nodeName)
        .build("fluencelabs/solver:latest")

      _ = logger.info("running {}", dockerRunCommand)

      containerId <- IO { Process(dockerRunCommand, new File(dockerWorkDir)).!! }
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

  private def clusterTupleToClusterFormed(
    clusterId: Bytes32,
    clusterTuple: ContractClusterTuple
  ): ClusterFormedEventResponse = {
    val artificialEvent = new ClusterFormedEventResponse()
    artificialEvent.clusterID = clusterId
    artificialEvent.storageHash = clusterTuple.getValue1
    artificialEvent.genesisTime = clusterTuple.getValue3
    artificialEvent.solverIDs = clusterTuple.getValue4
    artificialEvent.solverAddrs = clusterTuple.getValue5
    artificialEvent.solverPorts = clusterTuple.getValue6
    artificialEvent
  }

  private def configureLogging(): Unit = {
    PrintLoggerFactory.formatter = new DefaultPrefixFormatter(false, false, false)
    LoggerConfig.factory = PrintLoggerFactory()
    LoggerConfig.level = LogLevel.INFO
  }
}
