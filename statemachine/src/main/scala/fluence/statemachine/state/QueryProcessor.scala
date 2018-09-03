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

package fluence.statemachine.state

import cats.Monad
import cats.data.EitherT
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.statemachine.tree.TreePath
import fluence.statemachine.util.ClientInfoMessages

import scala.language.higherKinds

/**
 * Processor for ABCI `Query` requests.
 *
 * @param stateHolder [[TendermintStateHolder]] used to obtain Query state
 */
class QueryProcessor[F[_]](stateHolder: TendermintStateHolder[F])(implicit F: Monad[F]) extends slogging.LazyLogging {

  /**
   * Processes ABCI `Query` (Query thread).
   *
   * ABCI docs tells that `height` equal to 0 means the latest available height.
   * Providing arbitrary `height` as request parameter is currently forbidden.
   *
   * @param queryString target query
   * @param requestedHeight requested height
   * @param prove `Query` method parameter describing whether Merkle proof for result requested
   * @return data to construct `Query` response
   */
  def processQuery(queryString: String, requestedHeight: Long, prove: Boolean): F[QueryResponseData] =
    for {
      _ <- F.pure(logger.debug("Query: {}", queryString))

      parsedAndChecked <- (for {
        parsedQuery <- EitherT.fromEither[F](
          if (requestedHeight == 0) TreePath.parse(queryString)
          else Left(ClientInfoMessages.RequestingCustomHeightIsForbidden)
        )
        state <- stateHolder.queryState
        (height, queryState) = state
      } yield (height, queryState, parsedQuery)).value
    } yield
      parsedAndChecked match {
        case Left(message) => QueryResponseData(0, None, None, QueryCodeType.Bad, message)
        case Right((height, queryState, key)) =>
          val result = queryState.getValue(key)
          val (statusCode, info) =
            if (result.isDefined)
              (QueryCodeType.OK, ClientInfoMessages.SuccessfulQueryResponse)
            else
              (QueryCodeType.NotReady, ClientInfoMessages.ResultIsNotReadyYet)

          val proof = result.filter(_ => prove).map(_ => queryState.getProof(key).toHex)
          QueryResponseData(height, result, proof, statusCode, info)
      }
}

/**
 * A structure for aggregating data specific to building `Query` ABCI method response.
 *
 * @param height height corresponding to state for which result given
 * @param result requested result, if found
 * @param proof proof for the result, if requested and possible to be provided
 * @param code response code
 * @param info response message
 */
case class QueryResponseData(height: Long, result: Option[String], proof: Option[String], code: Int, info: String)

/**
 * Constants used as response codes for Query requests.
 */
object QueryCodeType {
  val OK: Int = 0 // Requested path exists. Result attached to the response.
  val NotReady: Int = 10 // Requested path is correct but not associated with value at height attached to the response.
  val Bad: Int = 1 // Bad path or bad height or something went wrong. Attached info contains details.
}
