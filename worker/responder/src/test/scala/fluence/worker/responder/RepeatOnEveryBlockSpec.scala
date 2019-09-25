package fluence.worker.responder

import cats.data.EitherT
import cats.syntax.apply._
import cats.syntax.flatMap._
import cats.syntax.applicative._
import cats.effect.{ContextShift, IO, Timer}
import cats.effect.concurrent.Ref
import fluence.bp.api.BlockProducer
import fluence.bp.embedded.{EmbeddedBlockProducer, SimpleBlock}
import fluence.bp.tx.{Tx, TxCode, TxResponse}
import fluence.effects.EffectError
import fluence.log.{Log, LogFactory}
import fluence.statemachine.api.StateMachine
import fluence.statemachine.api.StateMachine.Aux
import fluence.statemachine.api.command.TxProcessor
import fluence.statemachine.api.data.{StateHash, StateMachineStatus}
import fluence.statemachine.api.query.{QueryCode, QueryResponse}
import org.scalatest.WordSpec
import scodec.bits.ByteVector
import shapeless.HNil

import scala.concurrent.ExecutionContext.Implicits.global
import scala.language.postfixOps

case class TestErrorNoSuchTx(path: String) extends EffectError
case class TestErrorNotImplemented(msg: String) extends EffectError { initCause(new NotImplementedError(msg)) }
case class TestErrorMalformedTx(txData: Array[Byte]) extends EffectError

class RepeatOnEveryBlockSpec extends WordSpec {

  implicit private val ioTimer: Timer[IO] = IO.timer(global)
  implicit private val ioShift: ContextShift[IO] = IO.contextShift(global)

  implicit private val log: Log[IO] =
    LogFactory.forPrintln[IO](Log.Error).init("MasterNodeIntegrationSpec").unsafeRunSync()

  val txMap = Ref.of[IO, Map[String, Array[Byte]]](Map.empty)
  val heightRef = Ref.of[IO, Long](0)

  def embdeddedStateMachine(txMap: Ref[IO, Map[String, Array[Byte]]], heightRef: Ref[IO, Long]) =
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

  val embeddedBlockProducer: IO[BlockProducer.Aux[IO, SimpleBlock, HNil]] =
    (txMap, heightRef).mapN(embdeddedStateMachine) >>= EmbeddedBlockProducer.apply

  "on every block" should {
    "receive tx result" in {}
  }
}
