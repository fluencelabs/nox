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
import cats.syntax.flatMap._
import fluence.crypto.Crypto
import fluence.crypto.hash.JdkCryptoHasher
import fluence.log.{Log, LogFactory}
import fluence.statemachine.api.signals.BlockReceipt
import fluence.statemachine.error.StateMachineError
import fluence.statemachine.state.{MachineState, StateService}
import fluence.statemachine.vm.VmOperationInvoker
import fluence.vm.InvocationResult
import org.scalatest.{Matchers, WordSpec}
import scodec.bits.ByteVector

import scala.concurrent.ExecutionContext.Implicits.global
import scala.language.higherKinds

sealed trait Action
case object GetVmStateHash extends Action
case object GetReceipt extends Action
case class EnqueueVmHash(height: Long) extends Action
case class ExecutionState(actions: List[(Int, Action)] = Nil, counter: Int = 0) {
  def getVmStateHash() = copy(actions = actions :+ (counter, GetVmStateHash), counter = counter + 1)
  def getReceipt() = copy(actions = actions :+ (counter, GetReceipt), counter = counter + 1)
  def enqueueVmHash(height: Long) = copy(actions = actions :+ (counter, EnqueueVmHash(height)), counter = counter + 1)
  def clearActions() = copy(actions = Nil)
}

class StateServiceSpec extends WordSpec with Matchers {
  implicit private val timer = IO.timer(global)
  implicit private val shift = IO.contextShift(global)
  implicit private val log = LogFactory.forPrintln[IO]().init("AbciServiceSpec", level = Log.Error).unsafeRunSync()

  implicit val hasher: Crypto.Hasher[ByteVector, ByteVector] = {
    val bva = Crypto.liftFunc[ByteVector, Array[Byte]](_.toArray)
    val abv = Crypto.liftFunc[Array[Byte], ByteVector](ByteVector(_))
    bva.andThen[Array[Byte]](JdkCryptoHasher.Sha256).andThen(abv)
  }

  private def abciService(rs: List[BlockReceipt]) = {
    Apply[IO]
      .map2(Ref.of[IO, List[BlockReceipt]](rs), Ref.of[IO, ExecutionState](ExecutionState())) { (receipts, ref) =>
        val vmInvoker = new VmOperationInvoker[IO] {
          override def invoke(arg: Array[Byte]): EitherT[IO, StateMachineError, InvocationResult] =
            EitherT.rightT(InvocationResult(Array.empty, 0))

          override def vmStateHash(): EitherT[IO, StateMachineError, ByteVector] =
            EitherT.liftF(ref.update(_.getVmStateHash())).map(_ => ByteVector.empty)
        }

        val controlSignals = new TestControlSignals {
          override def getReceipt(height: Long): IO[BlockReceipt] =
            ref
              .update(_.getReceipt())
              .flatMap(_ => receipts.modify(l => (l.tail, l.head)))

          override def enqueueStateHash(height: Long, hash: ByteVector): IO[Unit] = ref.update(_.enqueueVmHash(height))
        }

        for {
          state ← Ref.of[IO, MachineState](MachineState())
          txCounter <- Ref.of[IO, Int](0)
          abci = new StateService[IO](state, vmInvoker, controlSignals, blockUploadingEnabled = true)
        } yield (abci, ref, state, txCounter)
      }
      .flatMap(identity)
  }

  private def checkCommit(
                           abci: StateService[IO],
                           ref: Ref[IO, ExecutionState],
                           abciState: Ref[IO, MachineState],
                           expectedActions: List[Action],
                           expectedHeight: Long
  ) = {
    Apply[IO].map2(abciState.get, ref.get) {
      case (abciState, executionState) =>
        abciState.height shouldBe expectedHeight

        val sorted = executionState.actions.sortBy(_._1).map(_._2)
        sorted should contain theSameElementsInOrderAs expectedActions
    }
  }

  def receipt(h: Long) = BlockReceipt(h, ByteVector(h.toString.getBytes()))

  "Abci Service" should {
    "retrieve receipts and store vmHash in order" in {
      val receipts = List(receipt(3))
      abciService(receipts).flatMap {
        case (abci, ref, abciState, txCounter) =>
          val makeCommit = abci.commit
          val incrementTx = txCounter.modify(c => (c + 1, c))
          val deliverTx = incrementTx.flatMap(i => abci.deliverTx(s"session/$i\ntxBody".getBytes()))
          val check = checkCommit(abci, ref, abciState, _, _)
          val clear = ref.update(_.clearActions())

          makeCommit *> check(List(GetVmStateHash, EnqueueVmHash(1)), 1) *> clear *>
            makeCommit *> check(List(GetVmStateHash, EnqueueVmHash(2)), 2) *> clear *>
            deliverTx *> makeCommit *> check(List(GetVmStateHash, GetReceipt, EnqueueVmHash(3)), 3)
      }.unsafeRunSync()
    }

    "work with stored receipts" in {
      // Receipts are retrieved only for non-empty blocks (and first 2 blocks always empty)
      val receipts = List(
        receipt(2),
        receipt(3),
        receipt(5)
      )

      abciService(receipts).flatMap {
        case (abci, ref, abciState, txCounter) =>
          val makeCommit = abci.commit
          val incrementTx = txCounter.modify(c => (c + 1, c))
          val deliverTx = incrementTx.flatMap(i => abci.deliverTx(s"session/$i\ntxBody".getBytes()))
          val check = checkCommit(abci, ref, abciState, _, _)
          val clear = ref.update(_.clearActions())

          makeCommit *> check(List(GetVmStateHash, EnqueueVmHash(1)), 1) *> clear *>
            makeCommit *> check(List(GetVmStateHash, EnqueueVmHash(2)), 2) *> clear *>
            deliverTx *> makeCommit *> check(List(GetVmStateHash, GetReceipt, EnqueueVmHash(3)), 3) *> clear *>
            deliverTx *> makeCommit *> check(List(GetVmStateHash, GetReceipt, EnqueueVmHash(4)), 4) *> clear *>
            makeCommit *> check(List(GetVmStateHash, EnqueueVmHash(5)), 5) *> clear *>
            deliverTx *> makeCommit *> check(List(GetVmStateHash, GetReceipt, EnqueueVmHash(6)), 6)
      }.unsafeRunSync()
    }
  }

  //TODO: Tx Ordering test
}
