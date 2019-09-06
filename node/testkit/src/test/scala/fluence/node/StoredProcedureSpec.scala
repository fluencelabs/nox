package fluence.node

import cats.effect.concurrent.{MVar, Ref}
import cats.effect.{ContextShift, IO, Resource, Timer}
import fluence.Eventually
import fluence.crypto.{Crypto, CryptoError}
import fluence.effects.tendermint.block.TestData
import fluence.effects.tendermint.block.data.Block
import fluence.log.LogFactory.Aux
import fluence.log.appender.PrintlnLogAppender
import fluence.log.{Log, LogFactory}
import fluence.node.workers.api.websocket.WorkerWebsocket.SubscriptionKey
import fluence.node.workers.subscription.StoredProcedureExecutor.TendermintResponse
import fluence.node.workers.subscription.{
  OkResponse,
  StoredProcedureExecutor,
  TendermintQueryResponse,
  TxAwaitError,
  WaitResponseService
}
import fluence.statemachine.data.Tx

import scala.concurrent.duration._
import org.scalatest.{EitherValues, Matchers, OptionValues, WordSpec}

import scala.concurrent.ExecutionContext.Implicits.global
import scala.util.Try

class StoredProcedureSpec extends WordSpec with Eventually with Matchers with OptionValues with EitherValues {

  implicit private val ioTimer: Timer[IO] = IO.timer(global)
  implicit private val ioShift: ContextShift[IO] = IO.contextShift(global)
  implicit private val logFactory: Aux[IO, PrintlnLogAppender[IO]] = LogFactory.forPrintln[IO](level = Log.Error)
  implicit private val log: Log.Aux[IO, PrintlnLogAppender[IO]] =
    logFactory.init("ResponseSubscriberSpec", level = Log.Off).unsafeRunSync()

  private def start() = {

    val hasher: Crypto.Hasher[Array[Byte], String] = Crypto.liftFuncEither(
      bytes ⇒
        Try {
          new String(bytes)
        }.toEither.left
          .map(err ⇒ CryptoError(s"Unexpected", Some(err)))
    )

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

        override def start(): Resource[IO, Unit] = throw new NotImplementedError("def start")
      }
      storedProcedureExecutor <- StoredProcedureExecutor.make[IO](
        tendermint.tendermint,
        tendermint.tendermint,
        waitResponseService,
        hasher
      )
      _ <- storedProcedureExecutor.start()
    } yield (storedProcedureExecutor, blocksQ)
  }

  private val block = Block(TestData.blockWithNullTxsResponse(1)).right.get

  private def startStream(
    stream: fs2.Stream[IO, TendermintResponse],
    events: Ref[IO, List[TendermintResponse]],
    eventsChecker: Ref[IO, Boolean],
    interruptChecker: Ref[IO, Boolean]
  ) = {
    stream.evalTap { e =>
      for {
        _ <- eventsChecker.update(_ => true)
        _ <- events.update(_ :+ e)
      } yield ()
    }.onFinalize(interruptChecker.update(_ => true))
      .compile
      .drain
      .unsafeRunAsyncAndForget()
  }

  "StoredProcedureExecutor" should {
    "be able to process subscribes, publish events through subscribers and close streams on unsubscribe" in {
      start().use {
        case (executor, blockQ) =>
          val data = Tx.Data(Array[Byte](1, 2, 3))
          val tx = new String(data.value)
          // generate blocks eventually
          fs2.Stream
            .awakeEvery[IO](50.milliseconds)
            .map(_ => block)
            .evalMap(blockQ.enqueue1)
            .compile
            .drain
            .unsafeRunAsyncAndForget()
          for {
            stream1EventsChecker <- Ref.of[IO, Boolean](false)
            stream1Events <- Ref.of[IO, List[TendermintResponse]](List.empty)
            stream2EventsChecker <- Ref.of[IO, Boolean](false)
            stream2Events <- Ref.of[IO, List[TendermintResponse]](List.empty)
            stream3EventsChecker <- Ref.of[IO, Boolean](false)
            stream3Events <- Ref.of[IO, List[TendermintResponse]](List.empty)
            stream1InterruptChecker <- Ref.of[IO, Boolean](false)
            stream2InterruptChecker <- Ref.of[IO, Boolean](false)
            stream3InterruptChecker <- Ref.of[IO, Boolean](false)

            stream1Id = SubscriptionKey("stream1", tx)
            stream23Id = SubscriptionKey("stream-23", tx)

            // create subscribers, 2 and 3 stream has the same id, so they should interrup together on unsubscribe
            stream1 <- executor.subscribe(stream1Id, data)
            stream2 <- executor.subscribe(stream23Id, data)
            stream3 <- executor.subscribe(stream23Id, data)

            _ = startStream(stream1, stream1Events, stream1EventsChecker, stream1InterruptChecker)
            _ = startStream(stream2, stream2Events, stream2EventsChecker, stream2InterruptChecker)
            _ = startStream(stream3, stream3Events, stream3EventsChecker, stream3InterruptChecker)

            // check if all streams have events
            _ <- eventually[IO]({ stream1EventsChecker.get.map { _ shouldBe true } }, 100.millis)
            _ <- eventually[IO]({ stream2EventsChecker.get.map { _ shouldBe true } }, 100.millis)
            _ <- eventually[IO]({ stream3EventsChecker.get.map { _ shouldBe true } }, 100.millis)

            // check if all streams have the same events
            _ <- eventually[IO](
              {
                for {
                  events1 <- stream1Events.get
                  events2 <- stream1Events.get
                  events3 <- stream3Events.get
                } yield {
                  events1.takeRight(10) should contain theSameElementsAs (events2.takeRight(10))
                  events1.takeRight(10) should contain theSameElementsAs (events3.takeRight(10))
                }
              },
              100.millis
            )
            _ <- executor.unsubscribe(stream1Id)

            // check if the first stream is finished after unsubscription
            _ <- eventually[IO](
              {
                for {
                  interruption1 <- stream1InterruptChecker.get
                  interruption2 <- stream2InterruptChecker.get
                  interruption3 <- stream3InterruptChecker.get
                } yield {
                  interruption1 shouldBe true
                  interruption2 shouldBe false
                  interruption3 shouldBe false
                }
              },
              100.millis
            )

            _ <- executor.unsubscribe(stream23Id)

            // check if the second and third stream is finished after unsubscription
            _ <- eventually[IO](
              {
                for {
                  interruption1 <- stream1InterruptChecker.get
                  interruption2 <- stream2InterruptChecker.get
                  interruption3 <- stream3InterruptChecker.get
                } yield {
                  interruption1 shouldBe true
                  interruption2 shouldBe true
                  interruption3 shouldBe true
                }
              },
              100.millis
            )
          } yield {}
      }.unsafeRunSync()
    }
  }
}
