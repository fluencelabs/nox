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
import java.util.concurrent.TimeUnit

import cats.Parallel
import cats.effect.concurrent.Deferred
import cats.effect.{ExitCode, IO, IOApp}
import fluence.ethclient.Deployer.{CLUSTERFORMED_EVENT, ClusterFormedEventResponse}
import fluence.ethclient.data.SolverInfo
import fluence.ethclient.helpers.RemoteCallOps._
import fluence.ethclient.helpers.Web3jConverters._
import io.circe.generic.auto._
import io.circe.syntax._
import org.web3j.abi.EventEncoder
import org.web3j.protocol.core.DefaultBlockParameterName
import org.web3j.protocol.core.methods.request.EthFilter

import scala.concurrent.duration._
import scala.sys.process._
import scala.util.Random

object MasterNodeApp extends IOApp {
  private val owner = "0x24b2285cfc8a68d1beec4f4282ee6016aebb8fc4"
  private val contractAddress = "0x2f99f35e068918daa5b559798290f57070ffdaec"

  private val bytes = stringToBytes32(Random.alphanumeric.take(10).mkString)

  private val filter =
    new EthFilter(DefaultBlockParameterName.EARLIEST, DefaultBlockParameterName.LATEST, contractAddress)
      .addSingleTopic(EventEncoder.encode(CLUSTERFORMED_EVENT))

  private def processClusterFormed(event: ClusterFormedEventResponse, solverInfo: SolverInfo): Unit = {
    val clusterData = clusterFormedEventToClusterData(event, solverInfo.validatorKey)

    println(clusterData.nodeInfo.asJson.spaces2)

    val clusterName = clusterData.nodeInfo.cluster.genesis.chain_id
    val vmCodeDirectory = "/Users/sergeev/git/fluencelabs/fluence/statemachine/docker/examples/vmcode-" + clusterData.code
    val nodeIndex = clusterData.nodeInfo.node_index.toInt
    val longTermKeyLocation = solverInfo.longTermLocation

    new File("/Users/sergeev/.fluence/nodes/" + clusterName + "/node" + clusterData.nodeInfo.node_index).mkdirs()
    Files.write(
      Paths.get(
        "/",
        "Users",
        "sergeev",
        ".fluence",
        "nodes",
        clusterName,
        "node" + clusterData.nodeInfo.node_index,
        "cluster_info.json"
      ),
      clusterData.nodeInfo.cluster.asJson.spaces2.getBytes
    )
    val clusterInfoJsonFile = "/Users/sergeev/.fluence/nodes/" + clusterName + "/node" + clusterData.nodeInfo.node_index + "/cluster_info.json"

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
        clusterInfoJsonFile,
        hostP2pPort,
        hostRpcPort,
        tmPrometheusPort,
        smPrometheusPort
      )
    println(Process(runString, new File("statemachine/docker")) !!)
  }

  override def run(args: List[String]): IO[ExitCode] =
    EthClient
      .makeHttpResource[IO]()
      .use { ethClient ⇒
        val par = Parallel[IO, IO.Par]
        // TODO: check number of params
        val solverInfo = new SolverInfo(args.head, args(1).toShort)

        for {
          _ ← IO(println("Launching w3j"))

          unsubscribe ← Deferred[IO, Either[Throwable, Unit]]

          version ← ethClient.clientVersion[IO]()
          _ = println(s"Client version: $version")

          _ ← par sequential par.apply.product(
            // Subscription stream
            par parallel ethClient
              .subscribeToLogsTopic[IO](contractAddress, EventEncoder.encode(CLUSTERFORMED_EVENT))
              .map(log ⇒ println(s"Log message: $log"))
              .interruptWhen(unsubscribe)
              .drain // drop the results, so that demand on events is always provided
              .onFinalize(IO(println("Subscription finalized")))
              .compile // Compile to a runnable, in terms of effect IO
              .drain, // Switch to IO[Unit]
            // Delayed unsubscribe
            par.parallel(for {
              contract <- ethClient.getDeployer[IO](contractAddress, owner)
              subscription = contract
                .clusterFormedEventObservable(filter)
                .take(120, TimeUnit.SECONDS)
                .last()
                .subscribe(x => processClusterFormed(x, solverInfo))

              txReceipt <- contract.addSolver(solverInfo.validatorKeyBytes32, solverInfo.addressBytes32).call[IO]

              //clusterFormedEvents <- contract
              //  .getEvent[IO, ClusterFormedEventResponse](_.getClusterFormedEvents(txReceipt))

              _ ← IO.sleep(120.seconds)

              _ = println("Going to unsubscribe")
              _ ← unsubscribe.complete(Right(()))
              //_ = println(s"${clusterFormedEvents.length}")
              _ = subscription.unsubscribe()
            } yield ())
          )
        } yield ()
      }
      .map { _ ⇒
        println("okay that's all")
        ExitCode.Success
      }
}
