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
      _ <- F.pure(logger.error("Query: {}", queryString))

      parsedAndChecked <- (for {
        parsedQuery <- EitherT.fromEither[F](
          if (requestedHeight == 0) TreePath.parse(queryString)
          else Left(ClientInfoMessages.RequestingCustomHeightIsForbidden)
        )
        state <- stateHolder.queryState
        (height, queryState) = state
      } yield (height, queryState, parsedQuery)).value
    } yield {
      println("PARSED AND CHECKED === " + parsedAndChecked)
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
