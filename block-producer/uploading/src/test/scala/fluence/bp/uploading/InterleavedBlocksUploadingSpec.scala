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
import fluence.effects.{Backoff, EffectError}
import fluence.effects.sttp.SttpEffect
import fluence.effects.tendermint.block
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.rpc.TestData
import fluence.effects.testkit.Timed
import fluence.log.{Log, LogFactory}
import org.scalatest.{Matchers, OptionValues, WordSpec}

import scala.concurrent.ExecutionContext.Implicits.global
import cats.data.EitherT
import cats.effect.IO
import cats.syntax.applicative._
import cats.syntax.apply._
import cats.syntax.functor._
import cats.effect.concurrent.Ref
import fluence.bp.uploading.controlled.ControlledBlockUploading
import fluence.effects.{Backoff, EffectError}
import fluence.effects.castore.StoreError
import fluence.effects.ipfs.{IpfsData, IpfsUploader}
import fluence.effects.sttp.SttpEffect
import fluence.effects.tendermint.block
import fluence.effects.tendermint.rpc.TestData
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.block.history.{BlockManifest, Receipt}
import fluence.effects.testkit.Timed
import fluence.log.{Log, LogFactory}
import fluence.statemachine.api.command.ReceiptBus
import fluence.statemachine.api.data.BlockReceipt
import org.scalatest.{Matchers, OptionValues, WordSpec}
import scodec.bits.ByteVector

import scala.concurrent.ExecutionContext.Implicits.global
import scala.concurrent.duration._

class InterleavedBlocksUploadingSpec extends WordSpec with Matchers with OptionValues with Timed {
  implicit private val timer = IO.timer(global)
  implicit private val shift = IO.contextShift(global)
  implicit private val log = LogFactory.forPrintln[IO]().init("BlockUploadingSpec", level = Log.Error).unsafeRunSync()
  implicit private val sttp = SttpEffect.stream[IO]
  implicit private val backoff: Backoff[EffectError] = Backoff.default[EffectError]

  private def singleBlock(height: Long) = TestData.parsedBlock(height)
  private def emptyBlock(height: Long) = Block(block.TestData.blockWithNullTxsResponse(height)).right.get

  /**
   * Generates and uploads a sequence of empty and non-empty blocks in which for each non-empty blocks there are number of emptyBlocks
   * @param blocks How much non-empty blocks there should be in the sequence
   * @param emptyBlocks Number of empty blocks before each non-empty block
   */
  def uploadBlocksWithEmptiesInterleaved(blocks: Int, emptyBlocks: Int): Unit = {
    val emptyBlocksTotal = blocks * emptyBlocks
    val blocksTotal = emptyBlocksTotal + blocks

    val bs = (1 to blocksTotal).map { h =>
      // insert N empty blocks before each non-empty block, where N = emptyBlocks
      if (h % (emptyBlocks + 1) == 0) singleBlock(h)
      else emptyBlock(h)
    }

    val emptyBlocksFollowedByNonEmpty =
      if (blocksTotal != 0 && bs.last.data.txs.exists(_.nonEmpty))
        // if last block is non-empty, all empty blocks are followed by non-empty ones
        emptyBlocksTotal
      else
        // if last block is empty, last N empty blocks aren't followed by non-empty; N = emptyBlocks
        emptyBlocksTotal - emptyBlocks

    ControlledBlockUploading
      .makeAndStart(bs)
      .use { ref =>
        eventually[IO](
          ref.get.map { state =>
            state.uploads shouldBe (blocks * 2 + emptyBlocksTotal)
            state.vmHashGet should contain theSameElementsInOrderAs (1 to blocksTotal)
            state.lastKnownHeight.value shouldBe 0L
            state.receipts.length shouldBe blocksTotal

            state.blockManifests.length shouldBe blocksTotal
            state.blockManifests.count(_.txsReceipt.isDefined) shouldBe blocks
            state.blockManifests.count(_.txsReceipt.isEmpty) shouldBe emptyBlocksTotal

            // check that all manifests for non-empty blocks were uploaded with N empty blocks; N = emptyBlocks
            state.blockManifests
              .filter(_.txsReceipt.isDefined)
              .foreach(_.emptyBlocksReceipts.length shouldBe emptyBlocks)
          }
        )
      }
      .unsafeRunSync()
  }

  "empty blocks interleaved with non-empty blocks" should {
    "be uploaded: 10 blocks, 2 empty blocks" in {
      uploadBlocksWithEmptiesInterleaved(blocks = 10, emptyBlocks = 2)
    }

    "be uploaded: 9 blocks, 3 empty blocks" in {
      uploadBlocksWithEmptiesInterleaved(blocks = 9, emptyBlocks = 3)
    }

    "be uploaded: 33 blocks, 5 empty blocks" in {
      uploadBlocksWithEmptiesInterleaved(blocks = 33, emptyBlocks = 5)
    }
  }
}
