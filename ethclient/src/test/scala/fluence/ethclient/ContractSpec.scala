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

import cats.effect.IO
import cats.effect.concurrent.MVar
import fluence.ethclient.Deployer.NewSolverEventResponse
import fluence.ethclient.helpers.RemoteCallOps._
import org.scalatest.{FlatSpec, Matchers}
import org.web3j.abi.EventEncoder
import org.web3j.abi.datatypes.Address
import org.web3j.abi.datatypes.generated.Bytes32
import org.web3j.protocol.core.methods.response.Log
import scodec.bits.ByteVector

import scala.concurrent.ExecutionContext.Implicits.global
import scala.util.Random

class ContractSpec extends FlatSpec with Matchers {
  private val url = sys.props.get("ethereum.url")
  private val client = EthClient.makeHttpResource[IO](url)
  private val client2 = EthClient.makeHttpResource[IO](url).allocate

  private def stringToBytes32(s: String) = {
    val byteValue = s.getBytes()
    val byteValueLen32 = new Array[Byte](32)
    System.arraycopy(byteValue, 0, byteValueLen32, 0, byteValue.length)
    new Bytes32(byteValueLen32)
  }

  def stringToHex(s: String) = {
    binaryToHex(s.getBytes())
  }

  def binaryToHex(b: Array[Byte]) = {
    ByteVector(b).toHex
  }

  "Ethereum client" should "receive an event" in {
    val str = Random.alphanumeric.take(10).mkString
    val bytes = stringToBytes32(str)
    val contractAddress = "0x9995882876ae612bfd829498ccd73dd962ec950a"
    val owner = "0x4180FC65D613bA7E1a385181a219F1DBfE7Bf11d"

    client.use { c =>
      for {
        event <- MVar.empty[IO, Log]
        unsubscribe ← c.subscribeToLogsTopic[IO, IO](
          contractAddress,
          EventEncoder.encode(Deployer.NEWSOLVER_EVENT),
          event.put
        )

        contract <- c.getDeployer[IO](contractAddress, owner)

        txReceipt <- contract.addAddressToWhitelist(new Address(owner)).call[IO]
        _ = assert(txReceipt.isStatusOK)

        txReceipt <- contract.addSolver(bytes, bytes).call[IO]
        _ = assert(txReceipt.isStatusOK)

        newSolverEvents <- contract.getEvent[IO, NewSolverEventResponse](
          _.getNewSolverEvents(txReceipt)
        )

        e <- event.take
        _ <- unsubscribe
      } yield {
        txReceipt.getLogs should contain(e)
        newSolverEvents.length shouldBe 1
        newSolverEvents.head.id shouldBe bytes
      }
    }.unsafeRunSync()
  }
}
