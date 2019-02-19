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
import fluence.ethclient.data.Log
import fluence.ethclient.syntax._
import fluence.ethclient.helpers.Web3jConverters._
import org.scalatest.{BeforeAndAfterAll, FlatSpec, Matchers}
import org.web3j.abi.datatypes.generated.{Bytes32, Uint16, Uint8}
import org.web3j.abi.datatypes.{Bool, DynamicArray}
import slogging.LazyLogging

import scala.collection.JavaConverters._
import scala.concurrent.ExecutionContext.Implicits.global
import scala.sys.process.{Process, ProcessLogger}
import scala.util.Random

class ClusterContractSpec extends FlatSpec with LazyLogging with Matchers with BeforeAndAfterAll {

  implicit private val ioTimer: Timer[IO] = IO.timer(global)
  implicit private val ioShift: ContextShift[IO] = IO.contextShift(global)

  private val url = sys.props.get("ethereum.url")
  private val client = EthClient.make[IO](url)

  val dir = new File("../bootstrap")
  def run(cmd: String): Unit = Process(cmd, dir).!(ProcessLogger(_ => ()))
  def runBackground(cmd: String): Unit = Process(cmd, dir).run(ProcessLogger(_ => ()))

  override protected def beforeAll(): Unit = {
    logger.info("bootstrapping npm")
    run("npm install")

    logger.info("starting Ganache")
    runBackground("npm run ganache")

    logger.info("deploying contracts to Ganache")
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

        data ← par sequential par.apply.productR(
          // Subscription stream
          par parallel ethClient
            .subscribeToLogsTopic[IO](
              contractAddress,
              Network.APPDEPLOYED_EVENT
            )
            .interruptWhen(unsubscribe)
            .head
            .evalMap[IO, Unit](event.put)
            .compile // Compile to a runnable, in terms of effect IO
            .drain // Switch to IO[Unit]
        )(
          // Delayed unsubscribe
          par.parallel {
            val contract = ethClient.getContract(contractAddress, owner, Network.load)

            for {
              _ <- contract
                .addApp(bytes, bytes, new Uint8(2), DynamicArray.empty("bytes32[]").asInstanceOf[DynamicArray[Bytes32]])
                .callUntilSuccess[IO]

              _ <- contract
                .addNode(
                  base64ToBytes32("RK34j5RkudeS0GuTaeJSoZzg/U5z/Pd73zvTLfZKU2w="),
                  nodeAddressToBytes24("192.168.0.1", "99d76509fe9cb6e8cd5fc6497819eeabb2498106"),
                  new Uint16(26056),
                  new Uint16(26057),
                  new Bool(false)
                )
                .callUntilSuccess[IO]

              txReceipt <- contract
                .addNode(
                  base64ToBytes32("LUMshgzPigL9jDYTCrMADlMyrJs1LIqfIlHCOlf7lOc="),
                  nodeAddressToBytes24("192.168.0.1", "1ef149b8ca80086350397bb6a02f2a172d013309"),
                  new Uint16(26156),
                  new Uint16(26157),
                  new Bool(false)
                )
                .callUntilSuccess[IO]

              _ = assert(txReceipt.isStatusOK)

              clusterFormedEvents <- IO(contract.getAppDeployedEvents(txReceipt).asScala.toList)

              // TODO: currently it takes more than 10 seconds to receive the event from the blockchain (Ganache), optimize
              e <- event.take
              _ <- unsubscribe.complete(Right(()))
            } yield (txReceipt, clusterFormedEvents, e)
          }
        )

        (txReceipt, clusterFormedEvents, e) = data

      } yield {
        txReceipt.getLogs.asScala.map(Log.apply) should contain(e)
        clusterFormedEvents.length shouldBe 1
      }
    }.unsafeRunSync()
  }
}
