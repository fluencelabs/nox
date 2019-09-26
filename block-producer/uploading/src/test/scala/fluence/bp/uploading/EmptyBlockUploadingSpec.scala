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
import fluence.bp.uploading.controlled.ControlledBlockUploading
import fluence.effects.sttp.SttpEffect
import fluence.effects.tendermint.block
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.rpc.TestData
import fluence.effects.testkit.Timed
import fluence.effects.{Backoff, EffectError}
import fluence.log.{Log, LogFactory}
import org.scalatest.{Matchers, OptionValues, WordSpec}

import scala.concurrent.ExecutionContext.Implicits.global
import scala.concurrent.duration._

class EmptyBlockUploadingSpec extends WordSpec with Matchers with OptionValues with Timed {
  implicit private val timer = IO.timer(global)
  implicit private val shift = IO.contextShift(global)
  implicit private val log = LogFactory.forPrintln[IO]().init("BlockUploadingSpec", level = Log.Error).unsafeRunSync()
  implicit private val sttp = SttpEffect.stream[IO]
  implicit private val backoff: Backoff[EffectError] = Backoff.default[EffectError]

  private def singleBlock(height: Long) = TestData.parsedBlock(height)
  private def emptyBlock(height: Long) = Block(block.TestData.blockWithNullTxsResponse(height)).right.get

  /**
   * Generates and uploads a sequence of blocks starting with a number of empty blocks followed by non-empty blocks
   * @param blocks Number of non-empty blocks
   * @param emptyBlocks Number of empty-blocks
   */
  def uploadBlockWithEmpties(blocks: Int, emptyBlocks: Int): Unit = {
    val empties = (1 to emptyBlocks).map(emptyBlock(_))
    val bs = (emptyBlocks + 1 to emptyBlocks + blocks).map(singleBlock(_))
    val allBlocks = empties ++ bs

    ControlledBlockUploading
      .makeAndStart(allBlocks)
      .use { ref =>
        eventually[IO](
          ref.get.map { state =>
            // for each non-empty block: upload txs + upload receipt; empty: upload receipt
            state.uploads shouldBe blocks * 2 + emptyBlocks

            // a single manifest for each block
            state.blockManifests.length shouldBe blocks + emptyBlocks
            // check that number of manifests for empty blocks is correct
            state.blockManifests.count(_.txsReceipt.isEmpty) shouldBe emptyBlocks
            // check that number of manifests for non-empty blocks is also correct
            state.blockManifests.count(_.txsReceipt.isDefined) shouldBe blocks
            if (blocks > 0) {
              // first non-empty block's manifest should contain receipts for the previous empty blocks
              state.blockManifests.find(_.txsReceipt.isDefined).value.emptyBlocksReceipts.length shouldBe emptyBlocks
            } else {
              state.blockManifests.find(_.txsReceipt.isDefined) should not be defined
            }

            // vm hash should be retrieved for every block
            state.vmHashGet should contain theSameElementsInOrderAs allBlocks.map(_.header.height)
            // we've started subscription from the very beginning
            state.lastKnownHeight.value shouldBe 0L

            // only receipts for non-empty blocks are sent
            state.receipts.length shouldBe blocks + emptyBlocks
          },
          period = 10.millis,
          maxWait = 1.second
        )
      }
      .unsafeRunSync()
  }

  "empty blocks + non-empty blocks" should {
    "be uploaded: 10 blocks + 9 empty blocks" in {
      uploadBlockWithEmpties(blocks = 10, emptyBlocks = 9)
    }

    "be uploaded: 0 blocks + 2 empty blocks" in {
      uploadBlockWithEmpties(blocks = 0, emptyBlocks = 2)
    }

    "be uploaded: 1 blocks + 2 empty blocks" in {
      uploadBlockWithEmpties(blocks = 1, emptyBlocks = 2)
    }

    "be uploaded: 13 blocks + 17 empty blocks" in {
      uploadBlockWithEmpties(blocks = 13, emptyBlocks = 17)
    }

    "be uploaded: 12 blocks + 16 empty blocks" in {
      uploadBlockWithEmpties(blocks = 12, emptyBlocks = 16)
    }
  }
}
