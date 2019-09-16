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

import cats.effect.concurrent.{MVar, Ref}
import cats.effect.{Concurrent, ContextShift, IO, Resource, Timer}
import cats.syntax.apply._
import cats.syntax.applicative._
import cats.syntax.functor._
import cats.syntax.monad._
import cats.syntax.traverse._
import cats.instances.list._
import fluence.Eventually
import fluence.effects.tendermint.rpc.TestData
import fluence.effects.tendermint.block.data.Block
import fluence.log.LogFactory.Aux
import fluence.log.appender.PrintlnLogAppender
import fluence.log.{Log, LogFactory}
import fluence.node.workers.api.websocket.WorkerWebsocket.SubscriptionKey
import fluence.node.workers.subscription.PerBlockTxExecutor.TendermintResponse
import fluence.node.workers.subscription._
import fluence.statemachine.api.tx.Tx
import org.scalatest.{EitherValues, Matchers, OptionValues, WordSpec}

import scala.concurrent.ExecutionContext.Implicits.global
import scala.concurrent.duration._
import scala.language.higherKinds

class PerBlockTxExecutorSpec extends WordSpec with Eventually with Matchers with OptionValues with EitherValues {

  implicit private val ioTimer: Timer[IO] = IO.timer(global)
  implicit private val ioShift: ContextShift[IO] = IO.contextShift(global)
  implicit private val logFactory: Aux[IO, PrintlnLogAppender[IO]] = LogFactory.forPrintln[IO](level = Log.Error)
  implicit private val log: Log.Aux[IO, PrintlnLogAppender[IO]] =
    logFactory.init("ResponseSubscriberSpec", level = Log.Off).unsafeRunSync()

  private def start() = {

    for {
      blocksQ <- Resource.liftF(fs2.concurrent.Queue.unbounded[IO, Block])
      tendermint <- Resource.liftF(TendermintTest[IO](blocksQ.dequeue))
      counter <- Resource.liftF(MVar.of[IO, Long](0L))
      waitResponseService = new WaitResponseService[IO] {
        override def sendTxAwaitResponse(tx: String, id: Option[String])(
          implicit log: Log[IO]
        ): IO[Either[TxAwaitError, TendermintQueryResponse]] =
          for {
            k <- counter.take
            _ <- counter.put(k + 1)
          } yield Right(OkResponse(Tx.Head("a", k), ""))
      }
      storedProcedureExecutor <- PerBlockTxExecutor.make[IO](
        tendermint.tendermint,
        tendermint.tendermint,
        waitResponseService
      )

    } yield (storedProcedureExecutor, blocksQ)
  }

  private val block = Block(TestData.block(1)).right.get

  case class StreamInfo[F[_]](events: F[List[TendermintResponse]], stopped: F[Boolean])
  private def startStream(stream: fs2.Stream[IO, TendermintResponse]) =
    for {
      events <- Ref.of[IO, List[TendermintResponse]](List.empty)
      stopped <- Ref.of[IO, Boolean](false)
      _ <- Concurrent[IO].start(
        stream
          .evalTap(e => events.update(_ :+ e))
          .onFinalize(stopped.update(_ => true))
          .compile
          .drain
      )
    } yield StreamInfo(events.get, stopped.get)

  "StoredProcedureExecutor" should {
    "be able to process subscribes, publish events through subscribers and close streams on unsubscribe" in {
      start().use {
        case (executor, blockQ) =>
          for {
            // generate blocks async-ly
            _ <- Concurrent[IO].start(
              fs2.Stream
                .awakeEvery[IO](50.milliseconds)
                .map(_ => block)
                .evalMap(blockQ.enqueue1)
                .compile
                .drain
            )
            tx = Tx.Data(Array[Byte](1, 2, 3))
            stream1Id = SubscriptionKey.generate("stream1", tx)
            stream23Id = SubscriptionKey.generate("stream-23", tx)

            // create subscribers, 2 and 3 stream has the same id, so they should interrup together on unsubscribe
            stream1 <- executor.subscribe(stream1Id, tx)
            stream2 <- executor.subscribe(stream23Id, tx)
            stream3 <- executor.subscribe(stream23Id, tx)

            info1 <- startStream(stream1)
            info2 <- startStream(stream2)
            info3 <- startStream(stream3)

            // check if all streams have events
            _ <- eventually[IO]({ info1.events.map(_ should not be empty) }, 100.millis)
            _ <- eventually[IO]({ info2.events.map(_ should not be empty) }, 100.millis)
            _ <- eventually[IO]({ info3.events.map(_ should not be empty) }, 100.millis)

            // check if all streams have the same events
            _ <- eventually[IO](
              for {
                events1 <- info1.events.map(_.takeRight(10))
                events2 <- info2.events.map(_.takeRight(10))
                events3 <- info3.events.map(_.takeRight(10))
              } yield {
                events1 should contain theSameElementsAs events2
                events2 should contain theSameElementsAs events3
              },
              100.millis
            )
            _ <- executor.unsubscribe(stream1Id)

            // check if the first stream is finished after unsubscription
            _ <- eventually[IO](
              (info1.stopped, info2.stopped, info3.stopped).mapN { case t => t shouldBe (true, false, false) },
              100.millis
            )

            _ <- executor.unsubscribe(stream23Id)

            // check if the second and third stream is finished after unsubscription
            _ <- eventually[IO](
              (info1.stopped, info2.stopped, info3.stopped).mapN { case t => t shouldBe (true, true, true) },
              100.millis
            )
          } yield {}
      }.unsafeRunSync()
    }
  }
}
