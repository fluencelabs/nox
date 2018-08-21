/*
 * Copyright (C) 2017  Fluence Labs Limited
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

import com.github.jtendermint.jabci.api.CodeType
import fluence.statemachine.QueryResponseData
import fluence.statemachine.tree.TreePath

/**
 * Processes `Query` ABCI request.
 *
 * @param stateHolder [[StateHolder]] used to obtain Query state
 */
class QueryProcessor(stateHolder: StateHolder) extends slogging.LazyLogging {

  /**
   * ABCI `Query` (Query thread).
   *
   * ABCI docs tells that `height` equal to 0 means the latest available height.
   * Providing arbitrary `height` as request parameter is currently forbidden.
   *
   * @param queryString target query
   * @param requestedHeight requested height
   * @param prove `Query` method parameter describing whether Merkle proof for result requested
   * @return data to construct `Query` response
   */
  def processQuery(queryString: String, requestedHeight: Long, prove: Boolean): QueryResponseData = {
    logger.info("Query: {}", queryString)

    val parsedAndChecked = for {
      _ <- if (requestedHeight == 0) Right(()) else Left("Requesting custom height is forbidden")
      parsedQuery <- TreePath.parse(queryString)
      state <- stateHolder.queryState
    } yield (state._1, state._2, parsedQuery)

    parsedAndChecked match {
      case Left(message) => QueryResponseData(0, None, None, CodeType.BAD, message)
      case Right((height, queryState, key)) =>
        val result = queryState.getValue(key)
        val proof = result.filter(_ => prove).map(_ => queryState.getProof(key).toHex)
        QueryResponseData(height, result, proof, CodeType.OK, "")
    }
  }
}
