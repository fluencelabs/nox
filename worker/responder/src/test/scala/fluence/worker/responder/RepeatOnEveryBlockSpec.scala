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
import cats.effect.{Concurrent, ContextShift, IO, Resource, Timer}
import cats.syntax.traverse._
import cats.instances.list._
import cats.syntax.apply._
import cats.syntax.flatMap._
import fluence.bp.api.BlockProducer
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
import fluence.worker.responder.resp.AwaitedResponse.OrError
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

  import WorkerResponderTestUtils._

  implicit private val ioTimer: Timer[IO] = IO.timer(global)
  implicit private val ioShift: ContextShift[IO] = IO.contextShift(global)
  implicit private val backoff: Backoff[EffectError] = Backoff.default[EffectError]

  implicit private val log: Log[IO] =
    LogFactory.forPrintln[IO](Log.Error).init("RepeatOnEveryBlockSpec").unsafeRunSync()

  private val txMap = Ref.of[IO, Map[String, Array[Byte]]](Map.empty)
  private val heightRef = Ref.of[IO, Long](0)

  private def embdeddedStateMachine(txMap: Ref[IO, Map[String, Array[Byte]]], heightRef: Ref[IO, Long]) =
    new StateMachine.ReadOnly[IO] {
      override def query(path: String)(implicit log: Log[IO]): EitherT[IO, EffectError, QueryResponse] =
        EitherT(
          for {
            m <- txMap.get
            h <- heightRef.get
          } yield
            m.get(path)
              .map(QueryResponse(h, _, QueryCode.Ok, ""))
              .toRight(TestErrorNoSuchTx(path))
        )

      override def status()(implicit log: Log[IO]): EitherT[IO, EffectError, StateMachineStatus] =
        EitherT.leftT(TestErrorNotImplemented("def status"))

    }.extend[TxProcessor[IO]](new TxProcessor[IO] {
      override def processTx(txData: Array[Byte])(implicit log: Log[IO]): EitherT[IO, EffectError, TxResponse] =
        for {
          path <- EitherT.fromOption[IO](Tx.splitTx(txData).map(_._1), TestErrorMalformedTx(txData))
          _ <- EitherT.right[EffectError](txMap.update(_.updated(path, txData)))
        } yield TxResponse(TxCode.OK, "")

      override def checkTx(txData: Array[Byte])(implicit log: Log[IO]): EitherT[IO, EffectError, TxResponse] =
        EitherT.rightT(TxResponse(TxCode.OK, ""))

      override def commit()(implicit log: Log[IO]): EitherT[IO, EffectError, StateHash] =
        EitherT.right(heightRef.modify(h => (h + 1, h + 1)).map(h => StateHash(h, ByteVector.empty)))
    })

  private val machineF = Resource.liftF((txMap, heightRef).mapN(embdeddedStateMachine))

  private val producer = (EmbeddedBlockProducer.apply[IO, TxProcessor[IO] :: HNil](_)).andThen(Resource.liftF(_))

  private val onEveryBlock = for {
    machine <- machineF
    producer <- producer(machine)
    awaitResponses <- AwaitResponses.make(producer, machine)
    sendAndWait = SendAndWait(producer, awaitResponses)
    onEveryBlock <- RepeatOnEveryBlock.make(producer, sendAndWait)
  } yield (onEveryBlock, producer: BlockProducer.AuxB[IO, SimpleBlock])

  "on every block" should {
    "receive tx result" in {
      val events = onEveryBlock.use {
        case (onEveryBlock, producer) =>
          produceBlocks(producer) >>= { fiber =>
            onEveryBlock
              .subscribe(SubscriptionKey("id", "hash"), Tx.Data("some tx".getBytes))
              .flatMap(s => s.take(10).compile.toList)
              .flatTap(_ => fiber.cancel)
          }
      }.unsafeRunTimed(2.seconds)

      events shouldBe defined
      events.value.length shouldBe 10
    }

    "receive tx results for several subs" in {
      val S = 5 // Number of subscriptions
      val N = 10 // Number of events expected from each subscription
      val events = onEveryBlock.use {
        case (onEveryBlock, producer) =>
          produceBlocks(producer) >>= { fiber =>
            // Subscribe and take N events
            def sub(s: Int): IO[fs2.Stream[IO, (Int, OrError)]] =
              onEveryBlock
                .subscribe(SubscriptionKey(s"id$s", "hash"), Tx.Data(s"$s some tx".getBytes))
                .map(_.take(N).map(s -> _))

            // Subscribe S times
            (1 to S).toList
              .traverse(sub)
              .flatMap(ss => fs2.Stream(ss: _*).parJoinUnbounded.compile.toList)
              .flatTap(_ => fiber.cancel)
          }
      }.unsafeRunTimed(2.seconds)

      events shouldBe defined
      events.value.length shouldBe N * S
      (1 to S).foreach { s =>
        events.value.count(_._1 == s) shouldBe N
      }
    }
  }
}
