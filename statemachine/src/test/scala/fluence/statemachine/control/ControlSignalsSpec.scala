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

package fluence.statemachine.control

import cats.Traverse
import cats.effect.IO
import cats.instances.list._
import fluence.log.{Log, LogFactory}
import fluence.statemachine.api.signals.BlockReceipt
import fluence.statemachine.control.signals.ControlSignals
import org.scalatest.{Matchers, OptionValues, WordSpec}
import scodec.bits.ByteVector

import scala.concurrent.ExecutionContext.Implicits.global
import scala.language.postfixOps

class ControlSignalsSpec extends WordSpec with Matchers with OptionValues {
  implicit private val timer = IO.timer(global)
  implicit private val shift = IO.contextShift(global)
  implicit private val log = LogFactory.forPrintln[IO]().init("control signals spec", level = Log.Error).unsafeRunSync()

  def receipt(height: Long) = BlockReceipt(height, ByteVector(height.toString.getBytes()))

  def checkReceipts(receipts: List[BlockReceipt], targetHeight: Long) = {
    ControlSignals
      .apply[IO]()
      .use { signals =>
        for {
          _ <- Traverse[List].traverse(receipts)(signals.enqueueReceipt)
          receipt <- signals.getReceipt(targetHeight)
        } yield {
          receipt.height shouldBe targetHeight
          new String(receipt.bytes.toArray).toLong shouldBe targetHeight
        }
      }
      .unsafeRunSync()
  }

  "control signals" should {
    "retrieve correct vmHash" in {
      val vmHashes = (1L to 10).map(h => (h, ByteVector.fromLong(h))).toList
      val targetHeight = 7
      ControlSignals
        .apply[IO]()
        .use { signals =>
          for {
            _ <- Traverse[List].traverse(vmHashes)(signals.enqueueVmHash _ tupled)
            vmHash <- signals.getVmHash(targetHeight)
          } yield {
            vmHash.height shouldBe targetHeight
            vmHash.hash.toLong() shouldBe targetHeight
          }
        }
        .unsafeRunSync()
    }

    "retrieve correct receipt" in {
      val receipts = (1L to 10).map(receipt).toList.reverse
      val targetHeight = 7
      checkReceipts(receipts, targetHeight)
    }

    "retrieve correct receipt from bad ordered queue" in {
      val receipts = (1L to 10).map(receipt).toList.reverse
      val targetHeight = 7
      checkReceipts(receipts, targetHeight)
    }
  }
}
