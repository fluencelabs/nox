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

package fluence.statemachine

import cats.Apply
import cats.data.EitherT
import cats.effect.IO
import cats.effect.concurrent.Ref
import cats.syntax.apply._
import cats.syntax.compose._
import fluence.crypto.Crypto
import fluence.crypto.hash.JdkCryptoHasher
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.block.history.Receipt
import fluence.effects.tendermint.rpc.http.{RpcError, RpcRequestErrored}
import fluence.log.{Log, LogFactory}
import fluence.statemachine.control.{BlockReceipt, ReceiptType}
import fluence.statemachine.error.StateMachineError
import fluence.statemachine.state.AbciState
import fluence.statemachine.vm.VmOperationInvoker
import org.scalatest.{Matchers, WordSpec}
import scodec.bits.ByteVector

import scala.concurrent.ExecutionContext.Implicits.global
import scala.language.higherKinds

sealed trait Action
case object GetVmStateHash extends Action
case object GetReceipt extends Action
case object PutVmHash extends Action
case object SetVmHash extends Action
case class ExecutionState(actions: List[(Int, Action)] = Nil, counter: Int = 0) {
  def getVmStateHash() = copy(actions = actions :+ (counter, GetVmStateHash), counter = counter + 1)
  def getReceipt() = copy(actions = actions :+ (counter, GetReceipt), counter = counter + 1)
  def putVmHash() = copy(actions = actions :+ (counter, PutVmHash), counter = counter + 1)
  def setVmHash() = copy(actions = actions :+ (counter, SetVmHash), counter = counter + 1)
  def clearActions() = copy(actions = Nil)
}

class AbciServiceSpec extends WordSpec with Matchers {
  implicit private val timer = IO.timer(global)
  implicit private val shift = IO.contextShift(global)
  implicit private val log = LogFactory.forPrintln[IO]().init("block uploading spec", level = Log.Warn).unsafeRunSync()

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

  private def abciService(rs: List[BlockReceipt]) = {
    Apply[IO]
      .map2(Ref.of[IO, List[BlockReceipt]](rs), Ref.of[IO, ExecutionState](ExecutionState())) { (receipts, ref) =>
        val vmInvoker = new VmOperationInvoker[IO] {
          override def invoke(arg: Array[Byte]): EitherT[IO, StateMachineError, Array[Byte]] =
            EitherT.rightT(Array.empty)

          override def vmStateHash(): EitherT[IO, StateMachineError, ByteVector] =
            EitherT.liftF(ref.update(_.getVmStateHash())).map(_ => ByteVector.empty)
        }

        val controlSignals = new TestControlSignals {
          override val receipt: IO[BlockReceipt] = ref
            .update(_.getReceipt())
            .flatMap(_ => receipts.modify(l => (l.tail, l.head)))

          override def putVmHash(hash: ByteVector): IO[Unit] = ref.update(_.putVmHash())
          override def setVmHash(hash: ByteVector): IO[Unit] = ref.update(_.setVmHash())
        }

        for {
          state ← Ref.of[IO, AbciState](AbciState())
          abci = new AbciService[IO](state, vmInvoker, controlSignals, tendermintRpc)
        } yield (abci, ref, state)
      }
      .flatMap(identity)
  }

  private def checkCommit(abci: AbciService[IO],
                          ref: Ref[IO, ExecutionState],
                          abciState: Ref[IO, AbciState],
                          expectedActions: List[Action],
                          expectedHeight: Long) = {
    Apply[IO].map2(abciState.get, ref.get) {
      case (abciState, executionState) =>
        abciState.height shouldBe expectedHeight

        val sorted = executionState.actions.sortBy(_._1).map(_._2)
        sorted should contain theSameElementsInOrderAs expectedActions
    }
  }

  def receipt(h: Long, t: ReceiptType.Value) = BlockReceipt(Receipt(h, ByteVector.fromLong(h)), t)

  "Abci Service" should {
    "retrieve receipts and store vmHash in order" in {
      val receipts = List(receipt(1, ReceiptType.New))
      abciService(receipts).flatMap {
        case (abci, ref, abciState) =>
          val makeCommit = abci.commit
          val check = checkCommit(abci, ref, abciState, _, _)
          val clear = ref.update(_.clearActions())

          makeCommit *> check(List(GetVmStateHash, PutVmHash), 1) *> clear *>
            makeCommit *> check(List(GetVmStateHash, GetReceipt, PutVmHash), 2)
      }.unsafeRunSync()
    }

    "work with stored receipts" in {
      val receipts = List(
        receipt(1, ReceiptType.Stored),
        receipt(2, ReceiptType.LastStored),
        receipt(3, ReceiptType.New),
        receipt(4, ReceiptType.New)
      )

      abciService(receipts).flatMap {
        case (abci, ref, abciState) =>
          val makeCommit = abci.commit
          val check = checkCommit(abci, ref, abciState, _, _)
          val clear = ref.update(_.clearActions())

          makeCommit *> check(List(GetVmStateHash, PutVmHash), 1) *> clear *>
            makeCommit *> check(List(GetVmStateHash, GetReceipt), 2) *> clear *>
            makeCommit *> check(List(GetVmStateHash, GetReceipt, SetVmHash), 3) *> clear *>
            makeCommit *> check(List(GetVmStateHash, GetReceipt, PutVmHash), 4) *> clear *>
            makeCommit *> check(List(GetVmStateHash, GetReceipt, PutVmHash), 5)
      }.unsafeRunSync()
    }
  }

  //TODO: Tx Ordering test
}
