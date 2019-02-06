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

import cats.Monad
import cats.effect.IO
import cats.syntax.functor._
import com.github.jtendermint.jabci.api._
import com.github.jtendermint.jabci.types.Request.ValueCase.DELIVER_TX
import com.github.jtendermint.jabci.types._
import com.google.protobuf.ByteString
import fluence.statemachine.control.{ControlSignals, DropPeer}
import fluence.statemachine.state.{Committer, QueryProcessor}
import fluence.statemachine.tx._
import fluence.statemachine.util.{ClientInfoMessages, Metrics, TimeLogger, TimeMeter}
import io.prometheus.client.Counter
import slogging.LazyLogging

import scala.language.higherKinds

/**
 * Entry point for incoming Tendermint ABCI requests
 *
 * Delegates processing logic to underlying processors, while unwrapping/wrapping Tendermint requests/results.
 */
class AbciHandler(
  private[statemachine] val committer: Committer[IO],
  private val queryProcessor: QueryProcessor[IO],
  private val txParser: TxParser[IO],
  private val checkTxStateChecker: TxStateDependentChecker[IO],
  private val deliverTxStateChecker: TxStateDependentChecker[IO],
  private val txProcessor: TxProcessor[IO],
  private val controlSignals: ControlSignals[IO]
) extends LazyLogging with ICheckTx with IDeliverTx with ICommit with IQuery with IEndBlock {

  private val queryCounter: Counter = Metrics.registerCounter("worker_query_count")
  private val queryProcessTimeCounter: Counter = Metrics.registerCounter("worker_query_process_time_sum")

  /**
   * Handler for `Commit` ABCI method (processed in Consensus thread).
   *
   * @param req `Commit` request data
   * @return `Commit` response data. Essentially, it is `app hash`.
   */
  override def requestCommit(req: RequestCommit): ResponseCommit =
    ResponseCommit.newBuilder
      .setData(committer.processCommit().unsafeRunSync())
      .build

  /**
   * Handler for `Query` ABCI method (processed in Query thread).
   *
   * @param req `Query` request data
   * @return `Query` response data
   */
  override def requestQuery(req: RequestQuery): ResponseQuery = {
    val queryTimeMeter = TimeMeter()
    val responseData = queryProcessor.processQuery(req.getPath, req.getHeight, req.getProve).unsafeRunSync()

    val queryDuration = queryTimeMeter.millisElapsed
    logger.debug("Query duration={} info={}", queryDuration, responseData.info)
    queryCounter.inc()
    queryProcessTimeCounter.inc(queryDuration)

    ResponseQuery.newBuilder
      .setCode(responseData.code)
      .setInfo(responseData.info)
      .setHeight(responseData.height)
      .setValue(ByteString.copyFromUtf8(responseData.result.getOrElse("")))
      .setProof(
        Proof
          .newBuilder()
          .addOps(ProofOp.newBuilder().setData(ByteString.copyFromUtf8(responseData.proof.getOrElse(""))))
      )
      .build
  }

  /**
   * Handler for `CheckTx` ABCI method (processed in Mempool thread).
   *
   * Tendermint requires that Mempool (`CheckTx`) and Consensus (`DeliverTx`, `Commit` and others) methods
   * are processed in different threads and against dedicated states. Therefore `CheckTx` processor is provided
   * with last committed state whereas `DeliverTx` works with 'real-time' mutable state.
   *
   * @param req `CheckTx` request data
   * @return `CheckTx` response data
   */
  override def requestCheckTx(req: RequestCheckTx): ResponseCheckTx = {
    val responseData = validateTx(req.getTx, txParser, checkTxStateChecker).unsafeRunSync()
    ResponseCheckTx.newBuilder
      .setCode(responseData.code)
      .setInfo(responseData.info)
      .setData(ByteString.copyFromUtf8(responseData.info))
      .build
  }

  /**
   * Handler for `DeliverTx` ABCI method (processed in Consensus thread).
   *
   * @param req `DeliverTx` request data
   * @return `DeliverTx` response data
   */
  override def receivedDeliverTx(req: RequestDeliverTx): ResponseDeliverTx = {
    val responseData = (for {
      validated <- validateTx(req.getTx, txParser, deliverTxStateChecker)

      processingTimeMeter = TimeMeter()
      _ <- validated.validatedTx match {
        case None => IO.unit
        case Some(tx) => txProcessor.processNewTx(tx)
      }
      processingDuration = processingTimeMeter.millisElapsed
      _ = logger.info("DeliverTx processTime={}", processingDuration)
    } yield validated).unsafeRunSync()
    ResponseDeliverTx.newBuilder
      .setCode(responseData.code)
      .setInfo(responseData.info)
      .setData(ByteString.copyFromUtf8(responseData.info))
      .build
  }

  /**
   * Parses [[Transaction]] from provided bytes and deduplicates it.
   *
   * This actions are common for `CheckTx` and `DeliverTx` processing, but deduplication is performed against
   * different states: Mempool state for `CheckTx`, Consensus state for `DeliverTx`.
   *
   * @param txBytes serialized transaction received from ABCI request called by Tendermint
   * @param txParser parser to extract transaction from received bytes
   * @param txStateChecker checker encapsulating some state used to check for duplicates and txs in closed sessions
   * @return validated transaction and data used to build a response
   */
  private def validateTx[F[_]](
    txBytes: ByteString,
    txParser: TxParser[F],
    txStateChecker: TxStateDependentChecker[F]
  )(implicit F: Monad[F]): F[TxResponseData] = {
    val validationTimeMeter = TimeMeter()
    for {
      validated <- (for {
        parsedTx <- txParser.parseTx(txBytes)
        checkedTx <- txStateChecker.check(parsedTx)
      } yield checkedTx).value.map {
        case Left(message) => TxResponseData(None, CodeType.BAD, message)
        case Right(tx) => TxResponseData(Some(tx), CodeType.OK, ClientInfoMessages.SuccessfulTxResponse)
      }

      duration = validationTimeMeter.millisElapsed
      latency = validated.validatedTx
        .flatMap(tx => tx.timestamp)
        .map(TimeMeter.millisFromPastToNow(_) - duration)
        .map(Math.max(0, _))
        .getOrElse(0L)

      _ = txStateChecker.collect(latency, duration)

      method = txStateChecker.method
      logMessage = () =>
        s"${TimeLogger.currentTime()} ${method.name()} latency=$latency validationTime=$duration $validated"

      verboseInfoLogNeeded = method == DELIVER_TX || validated.code != CodeType.OK
      _ = if (verboseInfoLogNeeded) logger.info(logMessage()) else logger.debug(logMessage())
    } yield validated
  }

  // At the end of block H, we can propose validator updates, they will be applied at block H+2
  // see https://github.com/tendermint/tendermint/blob/master/docs/spec/abci/abci.md#endblock
  override def requestEndBlock(
    req: RequestEndBlock
  ): ResponseEndBlock = {
    def dropValidator(drop: DropPeer) = {
      ValidatorUpdate
        .newBuilder()
        .setPubKey(
          PubKey
            .newBuilder()
            .setType(DropPeer.KEY_TYPE)
            .setData(ByteString.copyFrom(drop.validatorKey.toArray))
        )
        .setPower(0) // settings power to zero votes to remove the validator
    }
    controlSignals.dropPeers
      .use(
        drops =>
          IO.pure {
            drops
              .foldLeft(ResponseEndBlock.newBuilder()) {
                case (resp, drop) ⇒ resp.addValidatorUpdates(dropValidator(drop))
              }
              .build()
        }
      )
      .unsafeRunSync()
  }

}

/**
 * A structure for aggregating data specific to building `CheckTx`/`DeliverTx` ABCI response.
 *
 * @param validatedTx transaction, if successfully parsed and deduplicated
 * @param code response code
 * @param info response message
 */
private case class TxResponseData(validatedTx: Option[Transaction], code: Int, info: String) {
  override def toString: String = validatedTx match {
    case Some(tx) => s"Accepted ${tx.header}"
    case _ => s"Rejected $info"
  }
}
