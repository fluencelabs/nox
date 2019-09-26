package fluence.worker.responder

import cats.data.EitherT
import cats.effect.{Concurrent, ContextShift, Fiber, IO}
import fluence.bp.api.BlockProducer
import fluence.bp.embedded.SimpleBlock
import fluence.bp.tx.{Tx, TxResponse}
import fluence.effects.EffectError
import fluence.log.Log

import scala.util.Random

object WorkerResponderTestUtils {

  val sessionId: IO[String] = IO(Random.alphanumeric.take(8).mkString)

  def genTx(body: String): IO[Array[Byte]] =
    sessionId.map(sid => Tx(Tx.Head(sid, 0), Tx.Data(body.getBytes)).generateTx())
  def right[T](v: IO[T]): EitherT[IO, EffectError, T] = EitherT.right[EffectError](v)

  def sendTx(producer: BlockProducer.AuxB[IO, SimpleBlock],
             tx: String)(implicit log: Log[IO]): EitherT[IO, EffectError, TxResponse] =
    right(genTx(tx)).flatMap(producer.sendTx)

  def produceBlock(
    producer: BlockProducer.AuxB[IO, SimpleBlock]
  )(implicit log: Log[IO]): EitherT[IO, EffectError, TxResponse] =
    sendTx(producer, tx = "single tx produces block on embedded block producer")

  def produceBlocks(
    producer: BlockProducer.AuxB[IO, SimpleBlock]
  )(implicit log: Log[IO], contextShift: ContextShift[IO]): IO[Fiber[IO, Unit]] =
    Concurrent[IO].start(
      fs2
        .Stream(1)
        .repeat
        .evalMap(_ => produceBlock(producer).value)
        .compile
        .drain
    )
}
