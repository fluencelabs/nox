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
import fluence.EitherTSttpBackend
import fluence.effects.castore.StoreError
import fluence.effects.docker.DockerIO
import fluence.effects.docker.params.{DockerImage, DockerLimits}
import fluence.effects.ipfs.{IpfsData, IpfsUploader}
import fluence.effects.receipt.storage.{ReceiptStorage, ReceiptStorageError}
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.block.history.{BlockManifest, Receipt}
import fluence.effects.tendermint.{block, rpc}
import fluence.effects.tendermint.rpc.TendermintRpc
import fluence.effects.tendermint.rpc.websocket.TestTendermintRpc
import fluence.effects.{Backoff, EffectError}
import fluence.log.{Log, LogFactory}
import fluence.node.config.DockerConfig
import fluence.node.eth.state._
import fluence.node.workers.control.{ControlRpc, ControlRpcError}
import fluence.node.workers.status.WorkerStatus
import fluence.node.workers.tendermint.BlockUploading
import fluence.node.workers.tendermint.config.{ConfigTemplate, TendermintConfig}
import fluence.node.workers.{Worker, WorkerParams, WorkerServices}
import fluence.statemachine.control.ReceiptType
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

  val dockerIO = DockerIO.make[IO]()

  case class UploadingState(uploads: Int = 0,
                            vmHashGet: Int = 0,
                            receipts: Seq[(Receipt, ReceiptType.Value)] = Nil,
                            lastKnownHeight: Option[Long] = None,
                            blockManifests: Seq[BlockManifest] = Nil) {

    def upload[A: IpfsData](data: A) = data match {
      case d: ByteVector =>
        val manifests = BlockManifest.fromBytes(d).fold(_ => blockManifests, blockManifests :+ _)
        copy(uploads = uploads + 1, blockManifests = manifests)
      case _ =>
        copy(uploads = uploads + 1)
    }

    def vmHash() = copy(vmHashGet = vmHashGet + 1)

    def receipt(receipt: Receipt, rt: ReceiptType.Value) =
      copy(receipts = receipts :+ (receipt, rt))

    def subscribe(lastKnownHeight: Long) = copy(lastKnownHeight = Some(lastKnownHeight))

    def receiptTypes: Map[ReceiptType.Value, Int] = receipts.groupBy(_._2).mapValues(_.length)
  }

  /**
   * Starts BlockUploading for a given sequence on blocks and a given sequence of storedReceipts
   */
  private def startUploading(blocks: Seq[Block] = Nil, storedReceipts: Seq[Receipt] = Nil) = {
    Resource
      .liftF(Ref.of[IO, UploadingState](UploadingState()))
      .map { state =>
        def receiptStorage(id: Long) =
          Resource.pure[IO, ReceiptStorage[IO]](new ReceiptStorage[IO] {
            override val appId: Long = id

            override def put(height: Long, receipt: Receipt): EitherT[IO, ReceiptStorageError, Unit] = EitherT.pure(())
            override def get(height: Long): EitherT[IO, ReceiptStorageError, Option[Receipt]] = EitherT.pure(None)
            override def retrieve(from: Option[Long], to: Option[Long]): fs2.Stream[IO, (Long, Receipt)] =
              fs2.Stream.emits(storedReceipts.map(r => r.height -> r))
          })

        val ipfs = new IpfsUploader[IO] {
          override def upload[A: IpfsData](data: A)(implicit log: Log[IO]): EitherT[IO, StoreError, ByteVector] = {
            EitherT.liftF(state.update(_.upload(data)).map(_ => ByteVector.empty))
          }
        }

        val workerServices = new WorkerServices[IO] {
          override def tendermint: TendermintRpc[IO] = new TestTendermintRpc[IO] {
            override def subscribeNewBlock(
              lastKnownHeight: Long
            )(implicit log: Log[IO], backoff: Backoff[EffectError]): fs2.Stream[IO, Block] =
              fs2.Stream.eval(state.update(_.subscribe(lastKnownHeight))) >> fs2.Stream.emits(blocks)
          }

          override def control: ControlRpc[IO] = new TestControlRpc[IO] {
            override def sendBlockReceipt(receipt: Receipt,
                                          rType: ReceiptType.Value): EitherT[IO, ControlRpcError, Unit] =
              EitherT.liftF(state.update(_.receipt(receipt, rType)).void)

            override def getVmHash: EitherT[IO, ControlRpcError, ByteVector] =
              EitherT.liftF(state.update(_.vmHash()).map(_ => ByteVector.empty))
          }

          override def status(timeout: FiniteDuration): IO[WorkerStatus] =
            IO.raiseError(new NotImplementedError("def status worker status"))
        }

        (state, ipfs, workerServices, receiptStorage _)
      }
      .flatMap {
        case (state, ipfs, workerServices, receiptStorage) =>
          val worker: Resource[IO, Worker[IO]] =
            Worker.make[IO](appId, p2pPort, description, workerServices, (_: IO[Unit]) => IO.unit, IO.unit, IO.unit)

          worker.flatMap(BlockUploading.make[IO](ipfs, receiptStorage).start).map(_ => state)
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
    state.vmHashGet shouldBe blocks
    if (storedReceipts == 0) {
      state.lastKnownHeight.value shouldBe 0L
      state.receiptTypes.get(ReceiptType.Stored) should not be defined
      state.receiptTypes.get(ReceiptType.LastStored) should not be defined
    } else {
      state.lastKnownHeight.value shouldBe storedReceipts
      state.receiptTypes.getOrElse(ReceiptType.Stored, 0) shouldBe storedReceipts - 1
      state.receiptTypes.getOrElse(ReceiptType.LastStored, 0) shouldBe 1
    }

    state.receiptTypes.getOrElse(ReceiptType.New, 0) shouldBe blocks
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

    startUploading(empties ++ bs).use { ref =>
      eventually[IO](
        ref.get.map { state =>
          state.uploads shouldBe blocks * 2 + emptyBlocks
          state.vmHashGet shouldBe blocks + emptyBlocks
          state.lastKnownHeight.value shouldBe 0L
          state.receiptTypes.get(ReceiptType.Stored) should not be defined
          state.receiptTypes.get(ReceiptType.LastStored) should not be defined
          state.receiptTypes.getOrElse(ReceiptType.New, 0) shouldBe blocks + emptyBlocks

          state.blockManifests.length shouldBe blocks + emptyBlocks
          state.blockManifests.count(_.txsReceipt.isDefined) shouldBe blocks
          state.blockManifests.count(_.txsReceipt.isEmpty) shouldBe emptyBlocks
          state.blockManifests.find(_.txsReceipt.isDefined).value.emptyBlocksReceipts.length shouldBe emptyBlocks
        }
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
      // insert `emptyBlocks` empty blocks before each non-empty block
      if (h % (emptyBlocks + 1) == 0) singleBlock(h)
      else emptyBlock(h)
    }

    startUploading(bs).use { ref =>
      eventually[IO](
        ref.get.map { state =>
          state.uploads shouldBe (blocks * 2 + emptyBlocksTotal)
          state.vmHashGet shouldBe blocksTotal
          state.lastKnownHeight.value shouldBe 0L
          state.receiptTypes.get(ReceiptType.Stored) should not be defined
          state.receiptTypes.get(ReceiptType.LastStored) should not be defined
          state.receiptTypes.getOrElse(ReceiptType.New, 0) shouldBe blocksTotal

          state.blockManifests.length shouldBe blocksTotal
          state.blockManifests.count(_.txsReceipt.isDefined) shouldBe blocks
          state.blockManifests.count(_.txsReceipt.isEmpty) shouldBe emptyBlocksTotal

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
    "upload 1 + 2" in {
      uploadNBlocks(1, 2)
    }

    "upload 13 + 17" in {
      uploadNBlocks(13, 17)
    }

    "upload 12 + 16" in {
      uploadNBlocks(12, 16)
    }
  }

  "empty blocks" should {
    "be uploaded with an non-empty block (10 + 9)" in {
      uploadBlockWithEmpties(10, 9)
    }

    "1 + 2" in {
      uploadBlockWithEmpties(1, 2)
    }

    "13 + 17" in {
      uploadBlockWithEmpties(13, 17)
    }

    "12 + 16" in {
      uploadBlockWithEmpties(12, 16)
    }
  }

  "empty blocks interleaved" should {
    "be uploaded with non-empty blocks" in {
      uploadBlocksWithEmptiesInterleaved(10, 2)
    }

    "9 blocks with 3 empties before each" in {
      uploadBlocksWithEmptiesInterleaved(9, 3)
    }

    "33 blocks with 5 empties before each" in {
      uploadBlocksWithEmptiesInterleaved(33, 5)
    }
  }
}
