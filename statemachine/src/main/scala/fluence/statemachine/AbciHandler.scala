/*
 * Copyright (C) 2018  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

package fluence.statemachine

import cats.Monad
import cats.effect.IO
import cats.syntax.functor._
import com.github.jtendermint.jabci.api._
import com.github.jtendermint.jabci.types._
import com.google.protobuf.ByteString
import fluence.statemachine.state.{Committer, QueryProcessor}
import fluence.statemachine.tx._
import fluence.statemachine.util.ClientInfoMessages

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
  private val checkTxDuplicateChecker: TxDuplicateChecker[IO],
  private val deliverTxDuplicateChecker: TxDuplicateChecker[IO],
  private val txProcessor: TxProcessor[IO]
) extends ICheckTx with IDeliverTx with ICommit with IQuery {

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
    val responseData = queryProcessor.processQuery(req.getPath, req.getHeight, req.getProve).unsafeRunSync()
    ResponseQuery.newBuilder
      .setCode(responseData.code)
      .setInfo(responseData.info)
      .setHeight(responseData.height)
      .setValue(ByteString.copyFromUtf8(responseData.result.getOrElse("")))
      .setProof(ByteString.copyFromUtf8(responseData.proof.getOrElse("")))
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
    val responseData = validateTx(req.getTx, txParser, checkTxDuplicateChecker).unsafeRunSync()
    ResponseCheckTx.newBuilder
      .setCode(responseData.code)
      .setInfo(responseData.info)
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
      validated <- validateTx(req.getTx, txParser, deliverTxDuplicateChecker)
      _ <- validated.validatedTx match {
        case None => IO.unit
        case Some(tx) => txProcessor.processNewTx(tx)
      }
    } yield validated).unsafeRunSync()
    ResponseDeliverTx.newBuilder
      .setCode(responseData.code)
      .setInfo(responseData.info)
      .build
  }

  /**
   * Parses [[Transaction]] from provided bytes and deduplicates it.
   *
   * This actions are common for `CheckTx` and `DeliverTx` processing, but deduplication is performed against
   * different states: Mempool state for `CheckTx`, Consensus state for `DeliverTx`.
   *
   * @param txBytes serialized transaction received from ABCI request called by Tendermint
   * @param txDuplicateChecker duplicate checker encapsulating some state used to check for duplicates
   * @return validated transaction and data used to build a response
   */
  private def validateTx[F[_]: Monad](
    txBytes: ByteString,
    txParser: TxParser[F],
    txDuplicateChecker: TxDuplicateChecker[F]
  ): F[TxResponseData] = {
    (for {
      parsedTx <- txParser.parseTx(txBytes)
      uniqueTx <- txDuplicateChecker.deduplicate(parsedTx)
    } yield uniqueTx).value.map {
      case Left(message) => TxResponseData(None, CodeType.BAD, message)
      case Right(tx) => TxResponseData(Some(tx), CodeType.OK, ClientInfoMessages.SuccessfulTxResponse)
    }
  }
}

/**
 * A structure for aggregating data specific to building `CheckTx`/`DeliverTx` ABCI response.
 *
 * @param validatedTx transaction, if successfully parsed and deduplicated
 * @param code response code
 * @param info response message
 */
private case class TxResponseData(validatedTx: Option[Transaction], code: Int, info: String)
