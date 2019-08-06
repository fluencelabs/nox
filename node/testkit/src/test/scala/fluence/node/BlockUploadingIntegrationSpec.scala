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
import java.nio.charset.Charset
import java.nio.file.Paths

import cats.data.{EitherT, OptionT}
import cats.effect.concurrent.Ref
import cats.effect.{IO, Resource}
import cats.instances.list._
import cats.syntax.apply._
import cats.syntax.compose._
import cats.syntax.flatMap._
import cats.syntax.traverse._
import fluence.crypto.Crypto
import fluence.crypto.hash.JdkCryptoHasher
import fluence.effects.castore.StoreError
import fluence.effects.docker.DockerIO
import fluence.effects.docker.params.{DockerImage, DockerLimits}
import fluence.effects.ipfs.{IpfsData, IpfsUploader}
import fluence.effects.receipt.storage.{ReceiptStorage, ReceiptStorageError}
import fluence.effects.tendermint.block.data.{Base64ByteVector, Block}
import fluence.effects.tendermint.block.history.{BlockManifest, Receipt}
import fluence.effects.tendermint.rpc.TendermintRpc
import fluence.effects.tendermint.rpc.http.{RpcError, RpcRequestErrored}
import fluence.effects.tendermint.rpc.websocket.TestTendermintWebsocketRpc
import fluence.effects.tendermint.{block, rpc}
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
import fluence.statemachine.AbciService.TxResponse
import fluence.statemachine.control.{BlockReceipt, ControlSignals}
import fluence.statemachine.data.{Tx, TxCode}
import fluence.statemachine.error.StateMachineError
import fluence.statemachine.state.AbciState
import fluence.statemachine.vm.VmOperationInvoker
import fluence.statemachine.{AbciService, TestTendermintRpc}
import fluence.{EitherTSttpBackend, Eventually}
import fs2.concurrent.Queue
import io.circe.Json
import io.circe.parser.parse
import org.scalatest.{EitherValues, Matchers, OptionValues, WordSpec}
import scodec.bits.ByteVector

import scala.compat.Platform.currentTime
import scala.concurrent.ExecutionContext.Implicits.global
import scala.concurrent.duration._
import scala.language.higherKinds

class BlockUploadingIntegrationSpec extends WordSpec with Eventually with Matchers with OptionValues with EitherValues {
  implicit private val timer = IO.timer(global)
  implicit private val shift = IO.contextShift(global)
  implicit private val log = LogFactory.forPrintln[IO]().init("block uploading spec", level = Log.Error).unsafeRunSync()
  implicit private val sttp = EitherTSttpBackend[IO]()
  implicit private val backoff = Backoff.default[EffectError]

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

  val tendermintRpc = new TestTendermintRpc {
    override def block(height: Long, id: String): EitherT[IO, RpcError, Block] = {
      EitherT.leftT(RpcRequestErrored(777, "Block wasn't provided intentionally, for tests purpose"): RpcError)
    }
  }

  implicit val hasher: Crypto.Hasher[ByteVector, ByteVector] = {
    val bva = Crypto.liftFunc[ByteVector, Array[Byte]](_.toArray)
    val abv = Crypto.liftFunc[Array[Byte], ByteVector](ByteVector(_))
    bva.andThen[Array[Byte]](JdkCryptoHasher.Sha256).andThen(abv)
  }

  def control(): Resource[IO, (TestControlRpc[IO], ControlSignals[IO])] = {
    ControlSignals[IO]().map { signals =>
      val controlRpc = new TestControlRpc[IO] {
        override def sendBlockReceipt(receipt: Receipt): EitherT[IO, ControlRpcError, Unit] =
          EitherT.liftF(signals.enqueueReceipt(BlockReceipt(receipt)))

        override def getVmHash(height: Long): EitherT[IO, ControlRpcError, ByteVector] =
          EitherT.liftF(signals.getVmHash(height).map(_.hash))
      }

      (controlRpc, signals)
    }
  }

  private def abciService(controlSignals: ControlSignals[IO]) = {
    val vmInvoker = new VmOperationInvoker[IO] {
      override def invoke(arg: Array[Byte]): EitherT[IO, StateMachineError, Array[Byte]] =
        EitherT.rightT(Array.empty)

      override def vmStateHash(): EitherT[IO, StateMachineError, ByteVector] =
        EitherT.pure(ByteVector.empty)
    }

    for {
      state ← Ref.of[IO, AbciState](AbciState())
      abci = new AbciService[IO](state, vmInvoker, controlSignals, blockUploadingEnabled = true)
    } yield (abci, state)
  }

  private def startBlockUploading(controlRpc: ControlRpc[IO],
                                  blocksQ: fs2.concurrent.Queue[IO, Block],
                                  storedReceipts: Seq[Receipt] = Nil): Resource[IO, Unit] =
    Resource.liftF(Ref.of[IO, Option[BlockManifest]](None)).flatMap { manifestRef ⇒
      def receiptStorage(id: Long) =
        new ReceiptStorage[IO] {
          override val appId: Long = id

          override def put(height: Long,
                           receipt: Receipt)(implicit log: Log[IO]): EitherT[IO, ReceiptStorageError, Unit] =
            EitherT.pure(())
          override def get(height: Long)(implicit log: Log[IO]): EitherT[IO, ReceiptStorageError, Option[Receipt]] =
            EitherT.pure(None)
          override def retrieve(from: Option[Long],
                                to: Option[Long])(implicit log: Log[IO]): fs2.Stream[IO, (Long, Receipt)] =
            fs2.Stream.emits(storedReceipts.map(r => r.height -> r))
        }

      val ipfs = new IpfsUploader[IO] {
        override def upload[A: IpfsData](data: A)(implicit log: Log[IO]): EitherT[IO, StoreError, ByteVector] = {
          EitherT.pure(ByteVector.empty)
        }
      }

      val workerServices: WorkerServices[IO] = new WorkerServices[IO] {
        override def tendermint: TendermintRpc[IO] = new TestTendermintWebsocketRpc[IO] {
          override def subscribeNewBlock(
            lastKnownHeight: Long
          )(implicit log: Log[IO], backoff: Backoff[EffectError]): fs2.Stream[IO, Block] =
            blocksQ.dequeue
        }

        override val control: ControlRpc[IO] = controlRpc

        override def status(timeout: FiniteDuration): IO[WorkerStatus] =
          IO.raiseError(new NotImplementedError("def status worker status"))

        override def blockManifests: WorkerBlockManifests[IO] =
          new WorkerBlockManifests[IO](receiptStorage(appId), manifestRef)

        override def responseSubscriber: ResponseSubscriber[IO] =
          throw new NotImplementedError("def requestResponder")
      }

      val worker: Resource[IO, Worker[IO]] =
        Worker.make[IO](appId, p2pPort, description, workerServices, (_: IO[Unit]) => IO.unit, IO.unit, IO.unit)

      worker.flatMap(worker => BlockUploading[IO](enabled = true, ipfs).flatMap(_.start(worker)))
    }

  private def singleBlock(height: Long, txs: List[ByteVector]) = {
    val blockJson =
      parse(rpc.TestData.block(height)).right.get.hcursor
        .downField("result")
        .downField("data")
        .get[Json]("value")
        .right
        .get
    val block = Block(blockJson).right.get

    block.copy(data = block.data.copy(txs = Some(txs.map(Base64ByteVector))))
  }

  private def emptyBlock(height: Long) = Block(block.TestData.blockWithNullTxsResponse(height)).right.get

  private def generateTx(session: String, count: Int) =
    ByteVector
      .encodeString(
        s"""|$session/$count
            |this_should_be_a_llamadb_signature_but_it_doesnt_matter_for_this_test
            |1
            |INSERT INTO users VALUES(1, 'Sara', 23), (2, 'Bob', 19), (3, 'Caroline', 31), (4, 'Max', 27)
            |""".stripMargin
      )(Charset.defaultCharset())
      .right
      .value

  def start(): Resource[IO, (AbciService[IO], Ref[IO, AbciState], Queue[IO, Block])] =
    for {
      (controlRpc, controlSignals) <- control()
      (abciService, abciState) <- Resource.liftF(abciService(controlSignals))
      blocksQ <- Resource.liftF(fs2.concurrent.Queue.unbounded[IO, Block])
      _ <- startBlockUploading(controlRpc, blocksQ)
    } yield (abciService, abciState, blocksQ)

  "block uploading + abci service" should {
    "process blocks" in {
      val session = "folexshmolex"
      val emptyBlocks = (1L to 2).map(emptyBlock)
      var txCounter = 0
      def nextCount = { txCounter += 1; txCounter - 1 }
      val blocks = (3 to 10).map { h =>
        val txs = (1 to 3).map(_ => generateTx(session, nextCount)).toList
        singleBlock(h, txs)
      }

      val allBlocks = (emptyBlocks ++ blocks).toList

      start().use {
        case (abciService, abciState, blocksQ) =>
          allBlocks.traverse { block =>
            val checkResponses = (_: List[TxResponse]).foreach(_.code shouldBe TxCode.OK)
            val txs = block.data.txs.getOrElse(List.empty)
            val deliverTxs = txs.traverse(tx => abciService.deliverTx(tx.bv.toArray)).map(checkResponses)
            val commitBlock = deliverTxs *> abciService.commit
            val sendBlockToSubscription = blocksQ.enqueue1(block)

            val lastTx = OptionT.fromOption[IO](txs.lastOption).flatMap(tx => Tx.readTx[IO](tx.bv.toArray)).value
            val checkState = lastTx.flatMap { lastTx =>
              eventually[IO] {
                abciState.get.map { state =>
                  state.height shouldBe block.header.height

                  if (block.header.height > emptyBlocks.length) {
                    state.sessions.num shouldBe 1
                    lastTx shouldBe defined
                    state.sessions.data(session).nextNonce shouldBe lastTx.value.head.nonce + 1
                  }
                }
              }
            }

            commitBlock *> sendBlockToSubscription *> checkState
          }
      }.unsafeRunSync()
    }
  }
}
