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

import java.util.concurrent.TimeUnit

import cats.Parallel
import cats.effect.concurrent.Deferred
import cats.effect.{ExitCode, IO, IOApp}
import fluence.ethclient.Deployer.{CLUSTERFORMED_EVENT, ClusterFormedEventResponse}
import fluence.ethclient.helpers.RemoteCallOps._
import fluence.ethclient.helpers.Web3jConverters._
import org.web3j.abi.EventEncoder
import org.web3j.abi.datatypes.DynamicArray
import org.web3j.abi.datatypes.generated.Bytes32
import org.web3j.protocol.core.DefaultBlockParameterName
import org.web3j.protocol.core.methods.request.EthFilter

import scala.concurrent.duration._
import scala.util.Random

object MasterNodeApp extends IOApp {
  private val owner = "0x24b2285cfc8a68d1beec4f4282ee6016aebb8fc4"
  private val contractAddress = "0xe6d687c9d444714ced7cece2c028c660fa34c709"

  private val bytes = stringToBytes32(Random.alphanumeric.take(10).mkString)

  private val filter = new EthFilter(DefaultBlockParameterName.EARLIEST, DefaultBlockParameterName.LATEST, contractAddress)
    .addSingleTopic(EventEncoder.encode(CLUSTERFORMED_EVENT))

  private val solvers = List(
    ("RK34j5RkudeS0GuTaeJSoZzg/U5z/Pd73zvTLfZKU2w=", "192.168.0.1", 26056, "99d76509fe9cb6e8cd5fc6497819eeabb2498106"),
    ("LUMshgzPigL9jDYTCrMADlMyrJs1LIqfIlHCOlf7lOc=", "192.168.0.1", 26156, "1ef149b8ca80086350397bb6a02f2a172d013309"),
    ("PBgmsi6+B3f3/IJqHmIOq1rdL+2qKMQHPsy5qa1hxOs=", "192.168.0.1", 26256, "7e457eb2d99bf41db48ddbf6114f57a43342b943"),
    ("lj8SjF34KrPdBDExKAkxJVnSLB2h7y443Z+LiFCKyac=", "192.168.0.1", 26356, "09cc39ba51f1535f3a6be76aa662fb476fcae15e")
  )

  private def findSolverIndex(event: DynamicArray[Bytes32]): Int = ???

  private def processClusterFormed(event: ClusterFormedEventResponse): Unit = {
    val genesisText = b32DAtoGenesis(event.clusterID, event.solverIDs)
    val persistentPeers = b32DAtoPersistentPeers(event.solverAddrs)

    println(genesisText)
    println(persistentPeers)

//    val clusterName = b32ToChainId(event.clusterID)
//    val vm_code_directory
//    val nodeIndex = findSolverIndex(event.solverIDs)
//    val long_term_key_location
//    val cluster_info_json_file
//    val host_p2p_port
//    val host_rpc_port
//    val tm_prometheus_port
//    val sm_prometheus_port
  }

  override def run(args: List[String]): IO[ExitCode] =
    EthClient
      .makeHttpResource[IO]()
      .use { ethClient ⇒
        val par = Parallel[IO, IO.Par]
        val solverInitParams = solvers(args.headOption.map(_.toInt).getOrElse(2))

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
              subscription = contract.clusterFormedEventObservable(filter).take(30, TimeUnit.SECONDS).last().subscribe(x => {
                println(b32DAtoGenesis(x.clusterID, x.solverIDs))
                println(b32DAtoPersistentPeers(x.solverAddrs))
                processClusterFormed(x)
              })

              txReceipt <- contract.addSolver(base64ToBytes32(solverInitParams._1),
                solverAddressToBytes32(solverInitParams._2, solverInitParams._3.toShort, solverInitParams._4)).call[IO]

              clusterFormedEvents <- contract
                .getEvent[IO, ClusterFormedEventResponse](_.getClusterFormedEvents(txReceipt))

              _ ← IO.sleep(30.seconds)

              _ = println("Going to unsubscribe")
              _ ← unsubscribe.complete(Right(()))
              _ = println(s"${clusterFormedEvents.length}")
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
