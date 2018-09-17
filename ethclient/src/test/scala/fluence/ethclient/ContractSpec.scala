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
import org.web3j.abi.EventEncoder
import org.web3j.abi.datatypes.Address
import org.web3j.abi.datatypes.generated.Bytes32
import org.web3j.protocol.core.methods.response.Log
import utest._

import scala.collection.JavaConverters._
import scala.concurrent.ExecutionContext.Implicits.global
import scala.concurrent.{ExecutionContext, Future}
import scala.util.Random

object ContractSpec extends TestSuite {
  val ignored = true

  override def utestWrap(path: Seq[String], runBody: => Future[Any])(
    implicit ec: ExecutionContext
  ): Future[Any] = {
    if (!ignored) {
      super.utestWrap(path, runBody)(ec)
    } else {
      Future.successful(println("test is ignored"))
    }
  }

  private val url = sys.props.get("ethereum.url")
  private val client = EthClient.makeHttpResource[IO](url).allocate.map(_._1)

  private def stringToBytes32(s: String) = {
    val byteValue = s.getBytes()
    val byteValueLen32 = new Array[Byte](32)
    System.arraycopy(byteValue, 0, byteValueLen32, 0, byteValue.length)
    new Bytes32(byteValueLen32)
  }

  val tests: Tests = {
    Tests {
      "receive event" - {
        val str = Random.alphanumeric.take(10).mkString
        val bytes = stringToBytes32(str)
        val contractAddress = "0x29fae4a10580bc551b1c8c56d9d97f7d9088a252"
        val owner = "0x96dce7eb99848e3332e38663a1968836ba3c3b53"

        (for {
          c <- client
          event <- MVar.empty[IO, Log]
          unsubscribe ← c.subscribeToLogsTopic[IO, IO](
            contractAddress,
            EventEncoder.buildEventSignature("NewSolver(bytes32)"),
            event.put
          )
          contract <- c.getDeployer[IO](
            contractAddress,
            owner
          )
          _ <- contract.call[IO](_.addAddressToWhitelist(new Address(owner)))
          txReceipt <- contract.call[IO](_.addSolver(bytes, bytes))
          _ = assert(txReceipt.isStatusOK)
          newSolverEvents <- contract.getEvent[IO, NewSolverEventResponse](
            _.getNewSolverEvents(txReceipt)
          )
          e <- event.take
          _ <- unsubscribe
        } yield {
          assert(txReceipt.getLogs.asScala.contains(e))
          newSolverEvents.length ==> 1
          newSolverEvents.head.id ==> bytes
        }).unsafeToFuture()
      }
    }
  }
}
