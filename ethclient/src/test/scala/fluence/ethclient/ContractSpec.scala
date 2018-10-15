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

import cats.Parallel
import cats.effect.concurrent.{Deferred, MVar}
import cats.effect.{ContextShift, IO, Timer}
import fluence.ethclient.Deployer.{ClusterFormedEventResponse, NewSolverEventResponse}
import fluence.ethclient.helpers.RemoteCallOps._
import org.scalatest.{BeforeAndAfterAll, FlatSpec, Matchers}
import org.web3j.abi.EventEncoder
import org.web3j.abi.datatypes.Address
import org.web3j.abi.datatypes.generated.{Bytes32, Uint8}
import org.web3j.protocol.core.methods.response.Log
import scodec.bits.ByteVector
import slogging.LazyLogging

import scala.concurrent.ExecutionContext.Implicits.global
import scala.sys.process.{Process, ProcessLogger}
import scala.util.Random

class ContractSpec extends FlatSpec with LazyLogging with Matchers with BeforeAndAfterAll {

  implicit private val ioTimer: Timer[IO] = IO.timer(global)
  implicit private val ioShift: ContextShift[IO] = IO.contextShift(global)

  private val url = sys.props.get("ethereum.url")
  private val client = EthClient.makeHttpResource[IO](url)
  private val client2 = EthClient.makeHttpResource[IO](url)

  private def stringToBytes32(s: String): Bytes32 = {
    val byteValue = s.getBytes()
    val byteValueLen32 = new Array[Byte](32)
    System.arraycopy(byteValue, 0, byteValueLen32, 0, byteValue.length)
    new Bytes32(byteValueLen32)
  }

  private def stringToHex(s: String): String = {
    binaryToHex(s.getBytes())
  }

  private def binaryToHex(b: Array[Byte]): String = {
    ByteVector(b).toHex
  }

  val dir = new File("../bootstrap")
  def run(cmd: String): Unit = Process(cmd, dir).!(ProcessLogger(_ => ()))
  def runBackground(cmd: String): Unit = Process(cmd, dir).run(ProcessLogger(_ => ()))

  override protected def beforeAll(): Unit = {
    logger.info("bootstrapping npm")
    run("npm install")

    logger.info("starting Ganache")
    runBackground("npm run ganache")

    logger.info("deploying Deployer.sol Ganache")
    run("npm run migrate")
  }

  override protected def afterAll(): Unit = {
    logger.info("killing ganache")
    run("pkill -f ganache")
  }

  "Ethereum client" should "receive an event" in {
    val str = Random.alphanumeric.take(10).mkString
    val bytes = stringToBytes32(str)
    val contractAddress = "0x9995882876ae612bfd829498ccd73dd962ec950a"
    val owner = "0x4180FC65D613bA7E1a385181a219F1DBfE7Bf11d"

    client.use { ethClient =>
      val par = Parallel[IO, IO.Par]

      for {
        event <- MVar.empty[IO, Log]

        unsubscribe ← Deferred[IO, Either[Throwable, Unit]]

        data ← par sequential par.apply.product(
          // Subscription stream
          par parallel ethClient
            .subscribeToLogsTopic[IO](
              contractAddress,
              EventEncoder.encode(Deployer.CLUSTERFORMED_EVENT)
            )
            .interruptWhen(unsubscribe)
            .head
            .evalMap[IO, Unit](event.put)
            .compile // Compile to a runnable, in terms of effect IO
            .drain, // Switch to IO[Unit]

          // Delayed unsubscribe
          par.parallel(for {

            contract <- ethClient.getDeployer[IO](contractAddress, owner)

            txReceipt <- contract.addAddressToWhitelist(new Address(owner)).call[IO]
            _ = assert(txReceipt.isStatusOK)

            _ <- contract.addCode(bytes, bytes, new Uint8(1)).call[IO]

            txReceipt <- contract.addSolver(bytes, bytes).call[IO]
            _ = assert(txReceipt.isStatusOK)

            /*newSolverEvents <- contract.getEvent[IO, NewSolverEventResponse](
              _.getNewSolverEvents(txReceipt)
            )*/

            clusterFormedEvents <- contract.getEvent[IO, ClusterFormedEventResponse](
              _.getClusterFormedEvents(txReceipt)
            )

            // TODO: currently it takes more than 10 seconds to receive the event from the blockchain (Ganache), optimize
            e <- event.take
            _ <- unsubscribe.complete(Right(()))
          } yield (txReceipt, /*newSolverEvents, */ clusterFormedEvents, e))
        )

        (txReceipt, /*newSolverEvents, */ clusterFormedEvents, e) = data._2

      } yield {
        txReceipt.getLogs should contain(e)
        //newSolverEvents.length shouldBe 1
        //newSolverEvents.head.id shouldBe bytes
        clusterFormedEvents.length shouldBe 1
        println("ClusterFormedEvent: " + clusterFormedEvents.head.solverIDs.getValue)
      }
    }.unsafeRunSync()
  }
}
