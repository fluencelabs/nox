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

package fluence.statemachine.tx
import fluence.statemachine.StoreValue
import io.circe.syntax._
import io.circe.generic.auto._
import io.circe.parser.{parse => parseJson}


/**
 * TODO:
 *
 * @param status
 * @param invokedTxsCount
 * @param lastTxCounter
 */
case class SessionSummary(status: SessonStatus, invokedTxsCount: Long, lastTxCounter: Long) {

  /**
   * TODO:
   */
  def toStoreValue: StoreValue = this.asJson.noSpaces
}

object SessionSummary {

  /**
    * TODO:
    *
    * @param value
    */
  def fromStoreValue(value: StoreValue): Option[SessionSummary] =
    (for {
      parsedJson <- parseJson(value)
      summary <- parsedJson.as[SessionSummary]
    } yield summary).toOption
}

sealed abstract class SessonStatus

case object Active extends SessonStatus

sealed abstract class Closed extends SessonStatus

case object ExplicitlyClosed extends Closed

case object Failed extends Closed

case object Expired extends Closed
