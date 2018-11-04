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

import cats.Parallel
import cats.effect.concurrent.Deferred
import cats.effect.{ExitCode, IO, IOApp}
import fluence.ethclient.Deployer.{CLUSTERFORMED_EVENT, ClusterFormedEventResponse}
import fluence.ethclient.helpers.RemoteCallOps._
import fluence.ethclient.helpers.Web3jConverters._
import org.web3j.abi.EventEncoder
import org.web3j.abi.datatypes.Event
import org.web3j.protocol.core.DefaultBlockParameterName
import org.web3j.protocol.core.methods.request.EthFilter

import scala.concurrent.duration._
import scala.util.Random

object EthClientApp /*extends IOApp {
  val owner = "0x24b2285cfc8a68d1beec4f4282ee6016aebb8fc4"
  val contractAddress = "0xe6d687c9d444714ced7cece2c028c660fa34c709"

  val str = Random.alphanumeric.take(10).mkString
  val bytes = stringToBytes32(str)

  val filter = new EthFilter(DefaultBlockParameterName.EARLIEST, DefaultBlockParameterName.LATEST, contractAddress)
    .addSingleTopic(EventEncoder.encode(CLUSTERFORMED_EVENT))

  override def run(args: List[String]): IO[ExitCode] =
    EthClient
      .makeHttpResource[IO]()
      .use { ethClient ⇒
        val par = Parallel[IO, IO.Par]

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
              _ = contract.clusterFormedEventObservable(filter).subscribe(x => println("!!!" + x.log))

              txReceipt <- contract.addSolver(stringToBytes32("solver36"), stringToBytes32("addr36")).call[IO]

              clusterFormedEvents <- contract
                .getEvent[IO, ClusterFormedEventResponse](_.getClusterFormedEvents(txReceipt))

              _ ← IO.sleep(10.seconds)

              _ = println("Going to unsubscribe")
              _ ← unsubscribe.complete(Right(()))
              _ = println(s"${clusterFormedEvents.length}")
            } yield ())
          )
        } yield ()
      }
      .map { _ ⇒
        println("okay that's all")
        ExitCode.Success
      }
}
 */
