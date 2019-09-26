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

package fluence.bp.uploading

import cats.effect.IO
import fluence.effects.sttp.SttpEffect
import fluence.effects.tendermint.block
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.block.history.Receipt
import fluence.effects.tendermint.rpc.TestData
import fluence.effects.testkit.Timed
import fluence.effects.{Backoff, EffectError}
import fluence.log.{Log, LogFactory}
import org.scalatest.{Matchers, OptionValues, WordSpec}
import scodec.bits.ByteVector

import scala.concurrent.ExecutionContext.Implicits.global
import scala.concurrent.duration._

class BlockUploadingSpec extends WordSpec with Matchers with OptionValues with Timed {
  implicit private val timer = IO.timer(global)
  implicit private val shift = IO.contextShift(global)
  implicit private val log = LogFactory.forPrintln[IO]().init("BlockUploadingSpec", level = Log.Error).unsafeRunSync()
  implicit private val sttp = SttpEffect.stream[IO]
  implicit private val backoff: Backoff[EffectError] = Backoff.default[EffectError]

  private def singleBlock(height: Long) = TestData.parsedBlock(height)

  private def checkUploadState(state: UploadingState, blocks: Int, storedReceipts: Int) = {
    // For each block, we first upload txs, then upload the manifest, so 2 uploads for a block
    state.uploads shouldBe blocks * 2
    state.vmHashGet should contain theSameElementsInOrderAs (storedReceipts + 1 to blocks + storedReceipts)

    if (storedReceipts == 0) {
      state.lastKnownHeight.value shouldBe 0L
    } else {
      state.lastKnownHeight.value shouldBe storedReceipts
    }

    state.receipts.length shouldBe blocks + storedReceipts
  }

  /**
   * Simulates a situation where there is a given number of stored receipts followed by non-empty blocks
   */
  private def uploadNBlocks(blocks: Int, storedReceipts: Int = 0) = {
    val receipts = (1 to storedReceipts).map(h => Receipt(h, ByteVector.fromInt(h)))
    val bs = (storedReceipts + 1 to storedReceipts + blocks).map(singleBlock(_))

    ControlledBlockUploading
      .makeAndStart(bs, receipts)
      .use { ref =>
        eventually[IO](
          ref.get.map { state =>
            checkUploadState(state, blocks, storedReceipts)
          },
          period = 100.millis,
          maxWait = 2.second
        )
      }
      .unsafeRunSync()
  }

  "block uploading" should {
    "upload a single block" in {
      uploadNBlocks(1)
    }

    "upload 10 blocks" in {
      uploadNBlocks(10)
    }

    "upload 33 blocks" in {
      uploadNBlocks(33)
    }

    "upload 337 blocks" in {
      uploadNBlocks(337)
    }
  }

  "blocks + stored receipts" should {
    "upload 1 blocks + 2 stored receipts" in {
      uploadNBlocks(blocks = 1, storedReceipts = 2)
    }

    "upload 13 blocks + 17 stored receipts" in {
      uploadNBlocks(blocks = 13, storedReceipts = 17)
    }

    "upload 12 blocks + 16 stored receipts" in {
      uploadNBlocks(blocks = 12, storedReceipts = 16)
    }
  }
}
