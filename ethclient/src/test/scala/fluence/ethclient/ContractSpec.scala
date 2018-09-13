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

import cats.effect.IO
import cats.effect.concurrent.MVar
import utest._

import scala.concurrent.{ExecutionContext, Future}
import scala.concurrent.ExecutionContext.Implicits.global
import cats.instances.future._
import fluence.ethclient.Deployer.{ClusterFormedEventResponse, NewSolverEventResponse}
import org.web3j.abi.EventEncoder
import org.web3j.abi.datatypes.Address
import org.web3j.abi.datatypes.generated.Bytes32
import org.web3j.crypto.WalletUtils
import org.web3j.protocol.core.methods.response.Log
import scodec.bits.ByteVector

import scala.io.Source
import scala.util.Random
import collection.JavaConverters._

object ContractSpec extends TestSuite {

  override def utestWrap(path: Seq[String], runBody: => Future[Any])(
    implicit ec: ExecutionContext
  ): Future[Any] = {
    runBody
  }

  private val url = sys.props.get("ethereum.url")
  private val client = EthClient.makeHttpResource[IO](url).allocate.map(_._1)

  val keystore = Source.fromString(
    "{\"address\":\"96dce7eb99848e3332e38663a1968836ba3c3b53\",\"crypto\":{\"cipher\":\"aes-128-ctr\",\"ciphertext\":\"4cb93e14bb35364b2e5116df5401cbf23620e585fd583aa8f3d6a43a49b80137\",\"cipherparams\":{\"iv\":\"a00a643e8955ba139b2eb1f2cdea29b8\"},\"kdf\":\"scrypt\",\"kdfparams\":{\"dklen\":32,\"n\":262144,\"p\":1,\"r\":8,\"salt\":\"2a1378cfa289856d4a10251c24e5823620a352577b502b8a5a6765a022c86424\"},\"mac\":\"d1fff233d8333ad40826a006e6bf9520f678d69e10ffdc2f6b5699ad0961d5d9\"},\"id\":\"b15928d2-aae4-4059-ab2b-43c13b66c96c\",\"version\":3}"
  )

//  WalletUtils.loadCredentials("1234", )

  def stringToBytes32(s: String) = {
    val byteValue = s.getBytes()
    val byteValueLen32 = new Array[Byte](32)
    System.arraycopy(byteValue, 0, byteValueLen32, 0, byteValue.length)
    new Bytes32(byteValueLen32)
  }

  val tests = Tests {
    "receive event" - {
      val str = Random.nextString(10)
      val bytes = stringToBytes32(str)
      println(s"bytes are $str")
      val contractAddress = "0xe79e70e623ffe72860fb0cefed362b8537b7b8a4"
      val owner = "0x63f9068698f101d7ad41a0197461d94352a17d29"

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
