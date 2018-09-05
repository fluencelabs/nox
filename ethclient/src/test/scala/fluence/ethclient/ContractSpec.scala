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
import utest._

import scala.concurrent.{ExecutionContext, Future}
import scala.concurrent.ExecutionContext.Implicits.global
import cats.instances.future._
import org.web3j.abi.EventEncoder
import org.web3j.protocol.core.methods.response.Log

object ContractSpec extends TestSuite {

  override def utestWrap(path: Seq[String], runBody: => Future[Any])(
    implicit ec: ExecutionContext
  ): Future[Any] = {
    runBody
  }

  private val url = sys.props.get("ethereum.url")
  private val client = EthClient.makeHttpResource[IO](url).allocate.map(_._1)

  val tests = Tests {
    'test1 - {
      (for {
        c <- client
        event <- MVar.empty[IO, Log]
        unsubscribe ← c.subscribeToLogsTopic[IO, IO](
          "0xf93568cdc75b8849f4999bd3c8c6f931a14b258f",
          EventEncoder.buildEventSignature("NewSolver(bytes32)"),
          event.put
        )
        e <- event.take
        _ = e ==> e
        _ = throw new Exception(s"$c test1")
      } yield ()).unsafeToFuture()
    }

    'test3 - {
      val a = List[Byte](1, 2)
      a(10)
    }
  }
}
