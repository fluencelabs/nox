package fluence.node

import cats.effect.{ContextShift, IO, Resource, Timer}
import fluence.Eventually
import fluence.crypto.{Crypto, CryptoError}
import fluence.effects.tendermint.block.TestData
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.rpc.http.{RpcRequestErrored, TendermintHttpRpc}
import fluence.effects.tendermint.rpc.websocket.{TestTendermintWebsocketRpc, WebsocketConfig}
import fluence.log.{Log, LogFactory}
import fluence.node.workers.subscription.{
  ResponseSubscriber,
  RpcTxAwaitError,
  StoredProcedureExecutor,
  TendermintQueryResponse,
  TxAwaitError,
  WaitResponseService
}
import fluence.statemachine.data.Tx
import fs2.concurrent.Queue
import org.scalatest.{EitherValues, Matchers, OptionValues, WordSpec}

import scala.concurrent.ExecutionContext.Implicits.global
import scala.util.Try

class StoredProcedureSpec extends WordSpec with Eventually with Matchers with OptionValues with EitherValues {

  implicit private val ioTimer: Timer[IO] = IO.timer(global)
  implicit private val ioShift: ContextShift[IO] = IO.contextShift(global)
  implicit private val logFactory = LogFactory.forPrintln[IO](level = Log.Error)
  implicit private val log = logFactory.init("ResponseSubscriberSpec", level = Log.Off).unsafeRunSync()

  def start() = {

    val waitResponseService = new WaitResponseService[IO] {
      override def sendTxAwaitResponse(tx: String, id: Option[String])(
        implicit log: Log[IO]
      ): IO[Either[TxAwaitError, TendermintQueryResponse]] = IO.pure(Left(RpcTxAwaitError(RpcRequestErrored(1, "asd"))))

      override def start(): Resource[IO, Unit] = ???
    }

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
      storedProcedureExecutor <- Resource.liftF(
        StoredProcedureExecutor[IO](
          tendermint.tendermint,
          tendermint.tendermint,
          waitResponseService,
          hasher
        )
      )
      _ <- storedProcedureExecutor.start()
    } yield (storedProcedureExecutor, tendermint, blocksQ)
  }

  val block = Block(TestData.blockWithNullTxsResponse(1)).right.get

  "1" should {
    "2" in {

      val q = Queue.noneTerminated[IO, Int].unsafeRunSync()
      val broadcast = q.dequeue.broadcast
      val str1 = broadcast.take(1).flatten.evalTap(el => IO(println("elelele1: " + el))).compile
      val str2 = broadcast.take(1).flatten.evalTap(el => IO(println("elelele2: " + el))).compile
      val a = for {
        _ <- q.enqueue1(Option(1))
        _ = println("1111111")
        _ <- q.enqueue1(None)
        _ = println("22222")
        _ <- str1.drain
        _ = println("333333")
        _ <- str2.drain
        _ = println("44444")
      } yield {}

      a.unsafeRunSync()

      val result = start().use {
        case (executor, tendermint, blockQ) =>
          for {
            stream1 <- executor.subscribe(Tx.Data(Array[Byte](1, 2, 3)))
            stream2 <- executor.subscribe(Tx.Data(Array[Byte](1, 2, 3)))
            _ = println("111")
            _ <- blockQ.enqueue1(block)
            _ = println("2222")
            _ <- blockQ.enqueue1(block)
            _ = println("3333")
//            _ <- executor.unsubscribe(Tx.Data(Array[Byte](1, 2, 3)))
//            _ <- executor.unsubscribe(Tx.Data(Array[Byte](1, 2, 3)))
            res1 <- stream1.evalTap(el => IO(println("el1: " + el))).take(2).compile.toList
            _ = println("alalala")
            res2 <- stream2.evalTap(el => IO(println("el2: " + el))).take(2).compile.toList
            _ = println("4444")
          } yield {
            println("results = " + res1)
            println("results = " + res2)
          }
      }.unsafeRunSync()
    }
  }
}
