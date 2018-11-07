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
import java.nio.file.{Files, Paths}

import cats.Parallel
import cats.effect.concurrent.Deferred
import cats.effect.{ExitCode, IO, IOApp}
import fluence.ethclient.Deployer.{CLUSTERFORMED_EVENT, ClusterFormedEventResponse}
import fluence.ethclient.data.{DeployerContractConfig, SolverInfo}
import fluence.ethclient.helpers.JavaRxToFs2._
import fluence.ethclient.helpers.RemoteCallOps._
import fluence.ethclient.helpers.Web3jConverters._
import io.circe.generic.auto._
import io.circe.syntax._
import org.web3j.abi.EventEncoder
import org.web3j.protocol.core.methods.request.EthFilter
import org.web3j.protocol.core.{DefaultBlockParameter, DefaultBlockParameterName}
import slogging.MessageFormatter.DefaultPrefixFormatter
import slogging.{LazyLogging, LogLevel, LoggerConfig, PrintLoggerFactory}

import scala.language.postfixOps
import scala.sys.process._

object MasterNodeApp extends IOApp with LazyLogging {

  /**
   * Launches a single solver connecting to ethereum blockchain with Deployer contract.
   *
   * @param args 1st: Tendermint key location. 2nd: Tendermint p2p host IP. 3rd: Tendermint p2p port.
   */
  override def run(args: List[String]): IO[ExitCode] =
    EthClient
      .makeHttpResource[IO]()
      .use { ethClient ⇒
        val par = Parallel[IO, IO.Par]
        (for {
          solverInfo <- SolverInfo(args)
          config <- pureconfig.loadConfig[DeployerContractConfig]
          _ = configureLogging()
        } yield (solverInfo, config)) match {
          case Right((solverInfo, config)) =>
            for {
              unsubscribe ← Deferred[IO, Either[Throwable, Unit]]

              version ← ethClient.clientVersion[IO]()
              _ = logger.info("eth client version {}", version)

              contract <- ethClient.getDeployer[IO](config.deployerContractAddress, config.deployerContractOwnerAccount)

              currentBlock = ethClient.web3.ethBlockNumber().send().getBlockNumber
              filter = new EthFilter(
                DefaultBlockParameter.valueOf(currentBlock),
                DefaultBlockParameterName.LATEST,
                config.deployerContractAddress
              ).addSingleTopic(EventEncoder.encode(CLUSTERFORMED_EVENT))

              _ ← par sequential par.apply.product(
                // Subscription stream
                par parallel
                  contract
                    .clusterFormedEventObservable(filter)
                    .toFS2[IO]
                    .map(
                      x ⇒
                        if (processClusterFormed(x, solverInfo))
                          unsubscribe.complete(Right(()))
                        else
                          IO.unit
                    )
                    .map(_.unsafeRunSync())
                    .interruptWhen(unsubscribe)
                    .drain // drop the results, so that demand on events is always provided
                    .onFinalize(IO(logger.info("subscription finalized")))
                    .compile // Compile to a runnable, in terms of effect IO
                    .drain, // Switch to IO[Unit]
                // Delayed unsubscribe
                par.parallel(for {
                  _ <- contract.addSolver(solverInfo.validatorKeyBytes32, solverInfo.addressBytes32).call[IO]
                  _ <- unsubscribe.get
                  _ = logger.info("got unsubscribe")
                } yield ())
              )
            } yield ExitCode.Success
          case Left(value) =>
            logger.error("Error: {}", value)
            IO.pure(ExitCode.Error)
        }
      }

  /**
   * Tries to convert event information to cluster configuration and, in case of success, launches a single solver.
   *
   * @param event event from Ethereum Deployer contract
   * @param solverInfo information used to identify the current solver
   * @return whether this event is relevant to the passed SolverInfo
   */
  private def processClusterFormed(event: ClusterFormedEventResponse, solverInfo: SolverInfo): Boolean =
    clusterFormedEventToClusterData(event, solverInfo.validatorKey) match {
      case None => false
      case Some(clusterData) =>
        val clusterName = clusterData.nodeInfo.cluster.genesis.chain_id
        val vmCodeDir = System.getProperty("user.dir") + "/statemachine/docker/examples/vmcode-" + clusterData.code
        val nodeIndex = clusterData.nodeInfo.node_index.toInt
        val longTermKeyLocation = solverInfo.longTermLocation
        logger.info("preparing to join cluster '{}' as node {}", clusterName, nodeIndex)

        val homeDir = System.getProperty("user.home")
        val clusterInfoName = homeDir + "/.fluence/nodes/" + clusterName + "/node" + nodeIndex + "/cluster_info.json"
        val clusterInfoPath = Paths.get(clusterInfoName)

        new File(clusterInfoPath.getParent.toString).mkdirs()

        Files.write(clusterInfoPath, clusterData.nodeInfo.cluster.asJson.spaces2.getBytes)
        logger.info("cluster info written to {}", clusterInfoPath)

        val hostP2PPort = clusterData.hostP2PPort
        val hostRpcPort = clusterData.hostRpcPort
        val tmPrometheusPort = clusterData.tmPrometheusPort
        val smPrometheusPort = clusterData.smPrometheusPort

        val runString = s"""bash ./master-run-node.sh %s %s %d %s %s %d %d %d %d"""
          .format(
            clusterName,
            vmCodeDir,
            nodeIndex,
            longTermKeyLocation,
            clusterInfoName,
            hostP2PPort,
            hostRpcPort,
            tmPrometheusPort,
            smPrometheusPort
          )

        logger.info("running {}", runString)

        val containerId = Process(runString, new File("statemachine/docker")).!!
        logger.info("launched container {}", containerId)

        true
    }

  private def configureLogging(): Unit = {
    PrintLoggerFactory.formatter = new DefaultPrefixFormatter(false, false, false)
    LoggerConfig.factory = PrintLoggerFactory()
    LoggerConfig.level = LogLevel.INFO
  }
}
