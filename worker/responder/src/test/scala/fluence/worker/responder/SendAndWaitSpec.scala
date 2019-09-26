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

import cats.data.EitherT
import cats.syntax.flatMap._
import cats.effect.{ContextShift, IO, Resource, Timer}
import fluence.bp.api.BlockProducer
import fluence.bp.embedded.{EmbeddedBlockProducer, SimpleBlock}
import fluence.bp.tx.{TxCode, TxResponse}
import fluence.effects.{Backoff, EffectError}
import fluence.log.LogFactory.Aux
import fluence.log.appender.PrintlnLogAppender
import fluence.log.{Log, LogFactory}
import fluence.statemachine.api.StateMachine
import fluence.statemachine.api.command.TxProcessor
import fluence.statemachine.api.data.{StateHash, StateMachineStatus}
import fluence.statemachine.api.query.{QueryCode, QueryResponse}
import fluence.worker.Worker
import fluence.worker.responder.resp._
import org.scalatest._
import scodec.bits.ByteVector
import shapeless.{::, HNil}

import scala.concurrent.ExecutionContext.Implicits.global
import scala.concurrent.duration._
import scala.language.higherKinds

class SendAndWaitSpec extends WordSpec with Matchers with BeforeAndAfterAll with OptionValues with EitherValues {

  import WorkerResponderTestUtils._

  implicit private val ioTimer: Timer[IO] = IO.timer(global)
  implicit private val ioShift: ContextShift[IO] = IO.contextShift(global)
  implicit private val logFactory: Aux[IO, PrintlnLogAppender[IO]] = LogFactory.forPrintln[IO](level = Log.Error)
  implicit private val log: Log.Aux[IO, PrintlnLogAppender[IO]] =
    logFactory.init("ResponseSubscriberSpec", level = Log.Off).unsafeRunTimed(5.seconds).value
  implicit private val backoff: Backoff[EffectError] = Backoff.default[EffectError]

  private val maxBlockTries = 5

  private def embdeddedStateMachine(
    processTxResponse: Either[EffectError, TxResponse],
    queryResponse: Either[EffectError, QueryResponse]
  ): StateMachine.Aux[IO, TxProcessor[IO] :: HNil] =
    new StateMachine.ReadOnly[IO] {
      override def query(path: String)(implicit log: Log[IO]): EitherT[IO, EffectError, QueryResponse] =
        EitherT.fromEither(queryResponse)

      override def status()(implicit log: Log[IO]): EitherT[IO, EffectError, StateMachineStatus] =
        EitherT.leftT(TestErrorNotImplemented("def status"))

    }.extend[TxProcessor[IO]](new TxProcessor[IO] {
      override def processTx(txData: Array[Byte])(implicit log: Log[IO]): EitherT[IO, EffectError, TxResponse] =
        EitherT.fromEither(processTxResponse)

      override def checkTx(txData: Array[Byte])(implicit log: Log[IO]): EitherT[IO, EffectError, TxResponse] =
        EitherT.rightT(TxResponse(TxCode.OK, ""))

      override def commit()(implicit log: Log[IO]): EitherT[IO, EffectError, StateHash] =
        EitherT.right(IO(StateHash(1, ByteVector.empty)))
    })

  private val producer = (EmbeddedBlockProducer.apply[IO, TxProcessor[IO] :: HNil](_)).andThen(Resource.liftF(_))

  private val correctQueryResponse = QueryResponse(0, Array.emptyByteArray, QueryCode.Ok, "")

  private def sendAndWait(processTxResponse: Either[EffectError, TxResponse] = Right(TxResponse(TxCode.OK, "")),
                          queryResponse: Either[EffectError, QueryResponse] = Right(correctQueryResponse)) =
    for {
      machine <- Resource.liftF(IO(embdeddedStateMachine(processTxResponse, queryResponse)))
      producer <- producer(machine)
      worker = Worker(0L, machine, producer)
      awaitResponses <- AwaitResponses.make(worker, maxBlockTries)
      sendAndWait = SendAndWait(producer, awaitResponses)
    } yield (sendAndWait, producer: BlockProducer.AuxB[IO, SimpleBlock])

  private def tx(nonce: Int) =
    s"""|asdf/$nonce
        |this_should_be_a_llamadb_signature_but_it_doesnt_matter_for_this_test
        |1
        |INSERT INTO users VALUES(1, 'Sara', 23), (2, 'Bob', 19), (3, 'Caroline', 31), (4, 'Max', 27)
        |""".stripMargin

  def request(txCustom: Option[String] = None)(
    implicit log: Log[IO]
  ): ((SendAndWait[IO], BlockProducer.AuxB[IO, SimpleBlock])) => IO[Either[TxAwaitError, AwaitedResponse]] = {
    case (sendAndWait, blockProducer) =>
      produceBlocks(blockProducer) >>= { fiber =>
        sendAndWait.sendTxAwaitResponse(txCustom.getOrElse(tx(0)).getBytes()).value.flatTap(_ => fiber.cancel)
      }
  }

  object TxEffectError extends EffectError

  object QueryEffectError extends EffectError

  "MasterNode API" should {
    "return an RPC error, if broadcastTx returns an error RPC error" in {
      val result = sendAndWait(Left(TxEffectError)).use {
        request()
      }.unsafeRunTimed(5.seconds).value

      result should be('left)
      val error = result.left.get
      error shouldBe a[RpcTxAwaitError]
      error.asInstanceOf[RpcTxAwaitError].error shouldBe TxEffectError
    }

    "return an error, if broadcastTx returns an internal error" in {
      val result = sendAndWait(Right(TxResponse(TxCode.BAD, ""))).use {
        request()
      }.unsafeRunTimed(5.seconds).value

      result should be('left)
      val error = result.left.get
      error shouldBe a[TxInvalidError]
    }

    "return an error if tx sent is incorrect" in {
      val tx = "failed"
      val result = sendAndWait(Left(TxEffectError), Left(QueryEffectError)).use {
        request(Some(tx))
      }.unsafeRunTimed(5.seconds).value

      result should be('left)

      val error = result.left.get
      error shouldBe a[TxParsingError]
      error.asInstanceOf[TxParsingError].tx shouldBe tx.getBytes()
    }

    "return an error if query API from tendermint is not responded" in {

      val result = sendAndWait(queryResponse = Left(QueryEffectError)).use {
        request()
      }.unsafeRunTimed(5.seconds).value

      result should be('right)
      result.right.get shouldBe a[RpcErrorResponse]

      val error = result.right.get.asInstanceOf[RpcErrorResponse]
      error.error shouldBe QueryEffectError
    }

    "return a pending response, if tendermint cannot return response after some amount of blocks" in {
      val result =
        sendAndWait(queryResponse = Right(QueryResponse(1, Array.emptyByteArray, QueryCode.Pending, ""))).use {
          request()
        }.unsafeRunTimed(5.seconds).value

      result should be('right)
      result.right.get shouldBe a[TimedOutResponse]

      val error = result.right.get.asInstanceOf[TimedOutResponse]
      error.tries shouldBe maxBlockTries
    }

    // TODO: is it ok to return Dropped as OkResponse?
    "return a pending response, if tendermint return response with error code" in {
      val result =
        sendAndWait(queryResponse = Right(QueryResponse(1, Array.emptyByteArray, QueryCode.Dropped, ""))).use {
          request()
        }.unsafeRunTimed(5.seconds).value

      result should be('right)
      result.right.get shouldBe a[OkResponse]
    }

    "return OK result if tendermint is responded ok" in {
      val result = sendAndWait().use {
        request()
      }.unsafeRunTimed(5.seconds).value

      result should be('right)
      result.right.get shouldBe a[OkResponse]

      val ok = result.right.get.asInstanceOf[OkResponse]
      ok.response shouldBe correctQueryResponse.toResponseString()
    }
  }
}
