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

package fluence.worker.responder

import shapeless._
import cats.data.EitherT
import cats.effect.concurrent.Ref
import cats.effect.{ContextShift, IO, Resource, Timer}
import cats.syntax.apply._
import cats.syntax.applicative._
import cats.syntax.functor._
import cats.syntax.flatMap._
import fluence.bp.api.BlockProducer.Aux
import fluence.bp.embedded.{EmbeddedBlockProducer, SimpleBlock}
import fluence.bp.tx.{Tx, TxCode, TxResponse}
import fluence.effects.{Backoff, EffectError}
import fluence.log.{Log, LogFactory}
import fluence.statemachine.api.StateMachine
import fluence.statemachine.api.command.TxProcessor
import fluence.statemachine.api.data.{StateHash, StateMachineStatus}
import fluence.statemachine.api.query.{QueryCode, QueryResponse}
import fluence.worker.Worker
import fluence.worker.responder.repeat.{RepeatOnEveryBlock, SubscriptionKey}
import org.scalatest.{Matchers, OptionValues, WordSpec}
import scodec.bits.ByteVector
import shapeless.HNil

import scala.concurrent.ExecutionContext.Implicits.global
import scala.concurrent.duration._
import scala.language.postfixOps
import scala.util.Random

case class TestErrorNoSuchTx(path: String) extends EffectError
case class TestErrorNotImplemented(msg: String) extends EffectError { initCause(new NotImplementedError(msg)) }
case class TestErrorMalformedTx(txData: Array[Byte]) extends EffectError

class RepeatOnEveryBlockSpec extends WordSpec with OptionValues with Matchers {

  implicit private val ioTimer: Timer[IO] = IO.timer(global)
  implicit private val ioShift: ContextShift[IO] = IO.contextShift(global)
  implicit private val backoff: Backoff[EffectError] = Backoff.default[EffectError]

  implicit private val log: Log[IO] =
    LogFactory.forPrintln[IO](Log.Error).init("MasterNodeIntegrationSpec").unsafeRunSync()

  val txMap = Ref.of[IO, Map[String, Array[Byte]]](Map.empty)
  val heightRef = Ref.of[IO, Long](0)

  def embdeddedStateMachine(txMap: Ref[IO, Map[String, Array[Byte]]], heightRef: Ref[IO, Long]) =
    new StateMachine.ReadOnly[IO] {
      override def query(path: String)(implicit log: Log[IO]): EitherT[IO, EffectError, QueryResponse] =
        EitherT(
          for {
            _ <- IO(println(s"query $path"))
            m <- txMap.get
            h <- heightRef.get
          } yield m
            .get(path)
            .map(QueryResponse(h, _, QueryCode.Ok, ""))
            .toRight(TestErrorNoSuchTx(path))
        )

      override def status()(implicit log: Log[IO]): EitherT[IO, EffectError, StateMachineStatus] =
        EitherT.left(IO(println(s"status")).as(TestErrorNotImplemented("def status")))

    }.extend[TxProcessor[IO]](new TxProcessor[IO] {
      override def processTx(txData: Array[Byte])(implicit log: Log[IO]): EitherT[IO, EffectError, TxResponse] =
        for {
          _ <- EitherT.right(IO(println(s"processTx")))
          path <- EitherT.fromOption[IO](Tx.splitTx(txData).map(_._1), TestErrorMalformedTx(txData))
          _ <- EitherT.right[EffectError](txMap.update(_.updated(path, txData)))
        } yield TxResponse(TxCode.OK, "")

      override def checkTx(txData: Array[Byte])(implicit log: Log[IO]): EitherT[IO, EffectError, TxResponse] =
        EitherT.right(IO(println(s"checkTx")).as(TxResponse(TxCode.OK, "")))

      override def commit()(implicit log: Log[IO]): EitherT[IO, EffectError, StateHash] =
        EitherT.right(
          IO(println(s"commit")) *> heightRef.modify(h => (h + 1, h + 1)).map(h => StateHash(h, ByteVector.empty))
        )
    })

  val machineF = Resource.liftF((txMap, heightRef).mapN(embdeddedStateMachine))

  val producer = (EmbeddedBlockProducer.apply[IO, TxProcessor[IO] :: HNil](_)).andThen(Resource.liftF(_))

  val onEveryBlock = for {
    machine <- machineF
    producer <- producer(machine)
    worker = Worker(0L, machine, producer)
    awaitResponses <- AwaitResponses.make(worker)
    sendAndWait = SendAndWait(producer, awaitResponses)
    onEveryBlock <- RepeatOnEveryBlock.make(producer, sendAndWait)
  } yield (onEveryBlock, machine)

  val sessionId: IO[String] = IO(Random.alphanumeric.take(8).mkString)
  def genTx(body: String) = sessionId.map(sid => Tx(Tx.Head(sid, 0), Tx.Data(body.getBytes)).generateTx())
  def right[T](v: IO[T]) = EitherT.right[EffectError](v)

  def sendTx(machine: StateMachine.Aux[IO, TxProcessor[IO] :: HNil], tx: String) =
    right(genTx(tx)).flatMap(machine.command.processTx(_))

  def produceBlock(machine: StateMachine.Aux[IO, TxProcessor[IO] :: HNil]) =
    sendTx(machine, tx = "single tx produces block on embedded block producer")

  "on every block" should {
    "receive tx result" in {
      val events = onEveryBlock.use {
        case (onEveryBlock, machine) =>
          produceBlock(machine).value.flatTap(e => IO(s"produceBlock: $e")) *>
            onEveryBlock
              .subscribe(SubscriptionKey("id", "hash"), Tx.Data("some tx".getBytes))
              .flatTap(_ => IO("subcribed"))
              .flatTap(_ => produceBlock(machine).value)
              .flatMap(s => s.take(10).evalTap(e => IO(println(s"event: $e"))).compile.toList)
      }.unsafeRunTimed(10.seconds)

      println("events are: " + events)
      events shouldBe defined
      events.value.length shouldBe 10
    }
  }
}
