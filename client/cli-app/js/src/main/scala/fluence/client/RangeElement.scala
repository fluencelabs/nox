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

package fluence.client

import monix.execution.Scheduler
import monix.reactive.Observable
import org.scalajs.dom.document
import org.scalajs.dom.html.{Button, Input}

object RangeElement extends slogging.LazyLogging {

  /**
   * Add element with `range` logic.
   *
   * @param action Action, that will be processed on button click or by pressing `enter` key
   * @param resultField Field, that will be show the result of action.
   */
  def addrangeElement(action: (String, String) ⇒ Observable[(String, String)], resultField: Input)(
    implicit scheduler: Scheduler
  ): Unit = {

    val rangeButton = document.getElementById("range-submit").asInstanceOf[Button]
    val rangeFromInput = document.getElementById("range-from-input").asInstanceOf[Input]
    val rangeToInput = document.getElementById("range-to-input").asInstanceOf[Input]

    def rangeAction = {
      if (!rangeButton.disabled) {
        rangeButton.disabled = true
        val from = rangeFromInput.value
        val to = rangeToInput.value
        logger.info(s"Range get: from `$from` and to `$to`")
        val t = for {
          res ← action(from, to).toListL
        } yield {
          val resStr = res.mkString(", ")
          val printResult = s"Range get success. Result: $resStr"
          logger.info(printResult)
          resultField.value = printResult
          rangeFromInput.value = ""
          rangeToInput.value = ""
        }
        t.runAsync.onComplete(_ ⇒ rangeButton.disabled = false)
      }
    }

    rangeButton.onclick = mouseEvent ⇒ {
      rangeAction
    }

    rangeToInput.onkeypress = keyboardEvent ⇒ {
      if (keyboardEvent.charCode == 13) {
        rangeAction
      }
    }
  }
}
