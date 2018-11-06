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
import fluence.ethclient.data.SolverInfo
import fluence.ethclient.helpers.JavaRxToFs2._
import fluence.ethclient.helpers.RemoteCallOps._
import fluence.ethclient.helpers.Web3jConverters._
import io.circe.generic.auto._
import io.circe.syntax._
import org.web3j.abi.EventEncoder
import org.web3j.protocol.core.methods.request.EthFilter
import org.web3j.protocol.core.{DefaultBlockParameter, DefaultBlockParameterName}

import scala.sys.process._

object MasterNodeApp extends IOApp {
  private val owner = "0x24b2285cfc8a68d1beec4f4282ee6016aebb8fc4"
  private val contractAddress = "0x48419a38ed3cfed8e7106921d67470791a5aa268" // replace this with your contract address

  private def processClusterFormed(event: ClusterFormedEventResponse, solverInfo: SolverInfo): Boolean =
    clusterFormedEventToClusterData(event, solverInfo.validatorKey) match {
      case None => false
      case Some(clusterData) =>
        val clusterName = clusterData.nodeInfo.cluster.genesis.chain_id
        val vmCodeDirectory = System.getProperty("user.dir") +
          "/statemachine/docker/examples/vmcode-" + clusterData.code
        val nodeIndex = clusterData.nodeInfo.node_index.toInt
        val longTermKeyLocation = solverInfo.longTermLocation

        val homeDir = System.getProperty("user.home")
        val clusterInfoFileName = homeDir + "/.fluence/nodes/" + clusterName +
          "/node" + clusterData.nodeInfo.node_index + "/cluster_info.json"

        val clusterInfoPath = Paths.get(clusterInfoFileName)

        new File(clusterInfoPath.getParent.toString).mkdirs()
        Files.write(clusterInfoPath, clusterData.nodeInfo.cluster.asJson.spaces2.getBytes)

        val hostP2pPort = clusterData.hostP2pPort
        val hostRpcPort = clusterData.hostRpcPort
        val tmPrometheusPort = clusterData.tmPrometheusPort
        val smPrometheusPort = clusterData.smPrometheusPort

        val runString = s"""bash ./master-run-node.sh %s %s %d %s %s %d %d %d %d"""
          .format(
            clusterName,
            vmCodeDirectory,
            nodeIndex,
            longTermKeyLocation,
            clusterInfoFileName,
            hostP2pPort,
            hostRpcPort,
            tmPrometheusPort,
            smPrometheusPort
          )
        println(Process(runString, new File("statemachine/docker")) !!)

        true
    }

  override def run(args: List[String]): IO[ExitCode] =
    EthClient
      .makeHttpResource[IO]()
      .use { ethClient ⇒
        val par = Parallel[IO, IO.Par]
        val solverInfo = new SolverInfo(args.head, args(1).toShort)

        for {
          unsubscribe ← Deferred[IO, Either[Throwable, Unit]]

          version ← ethClient.clientVersion[IO]()
          _ = println(s"Client version: $version")

          contract <- ethClient.getDeployer[IO](contractAddress, owner)

          currentBlock = ethClient.web3.ethBlockNumber().send().getBlockNumber
          filter = new EthFilter(
            DefaultBlockParameter.valueOf(currentBlock),
            DefaultBlockParameterName.LATEST,
            contractAddress
          ).addSingleTopic(EventEncoder.encode(CLUSTERFORMED_EVENT))

          _ ← par sequential par.apply.product(
            // Subscription stream
            par parallel
              contract
                .clusterFormedEventObservable(filter)
                .toFS2[IO]
                .map(x ⇒ {
                  if (processClusterFormed(x, solverInfo))
                    unsubscribe.complete(Right(())).unsafeRunSync()
                })
                .interruptWhen(unsubscribe)
                .drain // drop the results, so that demand on events is always provided
                .onFinalize(IO(println("Subscription finalized")))
                .compile // Compile to a runnable, in terms of effect IO
                .drain, // Switch to IO[Unit]
            // Delayed unsubscribe
            par.parallel(for {
              _ <- contract.addSolver(solverInfo.validatorKeyBytes32, solverInfo.addressBytes32).call[IO]
              _ <- unsubscribe.get
            } yield ())
          )
        } yield ()
      }
      .map { _ ⇒
        println("okay that's all")
        ExitCode.Success
      }
}
