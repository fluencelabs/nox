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

package fluence.node

import java.net.InetAddress
import java.nio.file.Paths

import cats.data.EitherT
import cats.effect.concurrent.Ref
import cats.effect.{IO, Resource}
import cats.syntax.functor._
import cats.syntax.apply._
import fluence.{EitherTSttpBackend, Eventually}
import fluence.effects.castore.StoreError
import fluence.effects.docker.DockerIO
import fluence.effects.docker.params.{DockerImage, DockerLimits}
import fluence.effects.ipfs.{IpfsData, IpfsUploader}
import fluence.effects.receipt.storage.{ReceiptStorage, ReceiptStorageError}
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.block.history.{BlockManifest, Receipt}
import fluence.effects.tendermint.{block, rpc}
import fluence.effects.tendermint.rpc.TendermintRpc
import fluence.effects.tendermint.rpc.websocket.{TestTendermintWebsocketRpc, WebsocketConfig}
import fluence.effects.{Backoff, EffectError}
import fluence.log.{Log, LogFactory}
import fluence.node.config.DockerConfig
import fluence.node.eth.state._
import fluence.node.workers.control.{ControlRpc, ControlRpcError}
import fluence.node.workers.status.WorkerStatus
import fluence.node.workers.subscription.ResponseSubscriber
import fluence.node.workers.tendermint.block.BlockUploading
import fluence.node.workers.tendermint.config.{ConfigTemplate, TendermintConfig}
import fluence.node.workers.{Worker, WorkerBlockManifests, WorkerParams, WorkerServices}
import io.circe.Json
import io.circe.parser.parse
import org.scalatest.{Matchers, OptionValues, WordSpec}
import scodec.bits.ByteVector

import scala.compat.Platform.currentTime
import scala.concurrent.ExecutionContext.Implicits.global
import scala.concurrent.duration._

class BlockUploadingSpec extends WordSpec with Matchers with Eventually with OptionValues {
  implicit private val timer = IO.timer(global)
  implicit private val shift = IO.contextShift(global)
  implicit private val log = LogFactory.forPrintln[IO]().init("block uploading spec", level = Log.Warn).unsafeRunSync()
  implicit private val sttp = EitherTSttpBackend[IO]()
  implicit private val backoff: Backoff[EffectError] = Backoff.default[EffectError]

  private val rootPath = Paths.get("/tmp")

  val appId = 1L
  val p2pPort = 10001.toShort
  val description = "worker #1"
  val workerPeer = WorkerPeer(ByteVector.empty, "", 25000.toShort, InetAddress.getLocalHost, 0)
  val cluster = Cluster(currentTime.millis, Vector.empty, workerPeer)
  val app = App(123L, StorageRef(ByteVector.empty, StorageType.Ipfs), cluster)
  val dockerConfig = DockerConfig(DockerImage("fluencelabs/worker", "v0.2.0"), DockerLimits(None, None, None))
  val tmDockerConfig = DockerConfig(DockerImage("tendermint/tendermint", "v0.32.0"), DockerLimits(None, None, None))
  val tmConfig = TendermintConfig("info", 0, 0, 0, 0L, false, false, false, p2pPort, Seq.empty)
  val configTemplate = ConfigTemplate[IO](rootPath, tmConfig).unsafeRunSync()
  val params = WorkerParams(app, rootPath, rootPath, None, dockerConfig, tmDockerConfig, configTemplate)

  case class UploadingState(
    uploads: Int = 0,
    vmHashGet: Seq[Long] = Nil,
    receipts: Seq[Receipt] = Vector.empty,
    lastKnownHeight: Option[Long] = None,
    blockManifests: Seq[BlockManifest] = Nil
  ) {

    def upload[A: IpfsData](data: A) = data match {
      case d: ByteVector =>
        val manifests = BlockManifest.fromBytes(d).fold(_ => blockManifests, blockManifests :+ _)
        copy(uploads = uploads + 1, blockManifests = manifests)
      case _ =>
        copy(uploads = uploads + 1)
    }

    def vmHash(height: Long) = copy(vmHashGet = vmHashGet :+ height)

    def receipt(receipt: Receipt) =
      copy(receipts = receipts :+ receipt)

    def subscribe(lastKnownHeight: Long) = copy(lastKnownHeight = Some(lastKnownHeight))

  }

  /**
   * Starts BlockUploading for a given sequence on blocks and a given sequence of storedReceipts
   */
  private def startUploading(blocks: Seq[Block] = Nil, storedReceipts: Seq[Receipt] = Nil) = {
    Resource
      .liftF(
        (
          Ref.of[IO, UploadingState](UploadingState()),
          Ref.of[IO, Option[BlockManifest]](None)
        ).tupled
      )
      .map {
        case (state, manifestRef) =>
          def receiptStorage(id: Long) =
            new ReceiptStorage[IO] {
              override val appId: Long = id

              override def put(height: Long, receipt: Receipt)(
                implicit log: Log[IO]
              ): EitherT[IO, ReceiptStorageError, Unit] =
                EitherT.pure(())
              override def get(height: Long)(implicit log: Log[IO]): EitherT[IO, ReceiptStorageError, Option[Receipt]] =
                EitherT.pure(None)
              override def retrieve(from: Option[Long], to: Option[Long])(
                implicit log: Log[IO]
              ): fs2.Stream[IO, (Long, Receipt)] =
                fs2.Stream.emits(storedReceipts.map(r => r.height -> r))
            }

          val ipfs = new IpfsUploader[IO] {
            override def upload[A: IpfsData](data: A)(implicit log: Log[IO]): EitherT[IO, StoreError, ByteVector] = {
              EitherT.liftF(state.update(_.upload(data)).map(_ => ByteVector.empty))
            }
          }

          val workerServices = new WorkerServices[IO] {
            override def tendermint: TendermintRpc[IO] = new TestTendermintWebsocketRpc[IO] {
              override val websocketConfig: WebsocketConfig = WebsocketConfig()

              override def subscribeNewBlock(
                lastKnownHeight: Long
              )(implicit log: Log[IO], backoff: Backoff[EffectError]): fs2.Stream[IO, Block] =
                fs2.Stream.eval(state.update(_.subscribe(lastKnownHeight))) >> fs2.Stream.emits(blocks)
            }

            override def control: ControlRpc[IO] = new TestControlRpc[IO] {
              override def sendBlockReceipt(receipt: Receipt): EitherT[IO, ControlRpcError, Unit] =
                EitherT.liftF(state.update(_.receipt(receipt)).void)

              override def getVmHash(height: Long): EitherT[IO, ControlRpcError, ByteVector] =
                EitherT.liftF(state.update(_.vmHash(height)).map(_ => ByteVector.empty))
            }

            override def status(timeout: FiniteDuration): IO[WorkerStatus] =
              IO.raiseError(new NotImplementedError("def status worker status"))

            override def blockManifests: WorkerBlockManifests[IO] =
              new WorkerBlockManifests[IO](receiptStorage(appId), manifestRef)

            override def responseSubscriber: ResponseSubscriber[IO] =
              throw new NotImplementedError("def responseSubscriber")
          }

          (state, ipfs, workerServices)
      }
      .flatMap {
        case (state, ipfs, workerServices) =>
          val worker: Resource[IO, Worker[IO]] =
            Worker.make[IO](appId, p2pPort, description, workerServices, (_: IO[Unit]) => IO.unit, IO.unit, IO.unit)

          worker.flatMap(worker => BlockUploading[IO](enabled = true, ipfs).flatMap(_.start(worker))).map(_ => state)
      }
  }

  private def singleBlock(height: Long) = {
    val blockJson =
      parse(rpc.TestData.block(height)).right.get.hcursor
        .downField("result")
        .downField("data")
        .get[Json]("value")
        .right
        .get
    Block(blockJson).right.get
  }

  private def emptyBlock(height: Long) = Block(block.TestData.blockWithNullTxsResponse(height)).right.get

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

    startUploading(bs, receipts).use { ref =>
      eventually[IO](
        ref.get.map { state =>
          checkUploadState(state, blocks, storedReceipts)
        },
        period = 10.millis,
        maxWait = 1.second
      )
    }.unsafeRunSync()
  }

  /**
   * Generates and uploads a sequence of blocks starting with a number of empty blocks followed by non-empty blocks
   * @param blocks Number of non-empty blocks
   * @param emptyBlocks Number of empty-blocks
   */
  private def uploadBlockWithEmpties(blocks: Int, emptyBlocks: Int) = {
    val empties = (1 to emptyBlocks).map(emptyBlock(_))
    val bs = (emptyBlocks + 1 to emptyBlocks + blocks).map(singleBlock(_))
    val allBlocks = empties ++ bs

    startUploading(allBlocks).use { ref =>
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
    }.unsafeRunSync()
  }

  /**
   * Generates and uploads a sequence of empty and non-empty blocks in which for each non-empty blocks there are number of emptyBlocks
   * @param blocks How much non-empty blocks there should be in the sequence
   * @param emptyBlocks Number of empty blocks before each non-empty block
   */
  private def uploadBlocksWithEmptiesInterleaved(blocks: Int, emptyBlocks: Int) = {
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

    startUploading(bs).use { ref =>
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
          state.blockManifests.filter(_.txsReceipt.isDefined).foreach(_.emptyBlocksReceipts.length shouldBe emptyBlocks)
        }
      )
    }.unsafeRunSync()
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
