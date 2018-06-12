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

import monix.eval.Task
import monix.execution.Scheduler
import org.scalajs.dom.document
import org.scalajs.dom.html.{Button, Input}

object GetElement extends slogging.LazyLogging {

  /**
   * Add element with `get` logic.
   *
   * @param action Action, that will be processed on button click or by pressing `enter` key
   * @param resultField Field, that will be show the result of action.
   */
  def addGetElement(action: String ⇒ Task[Option[String]], resultField: Input)(
    implicit scheduler: Scheduler
  ): Unit = {

    val getButton = document.getElementById("get-submit").asInstanceOf[Button]
    val getInput = document.getElementById("get-input").asInstanceOf[Input]

    def getAction = {
      if (!getButton.disabled) {
        getButton.disabled = true
        val key = getInput.value
        logger.info(s"Get key: $key")
        val t = for {
          res ← action(key).map(Utils.prettyResult)
        } yield {
          val printResult = s"Get operation success. Value: $res"
          logger.info(printResult)
          resultField.value = printResult
          getInput.value = ""
        }
        t.runAsync.onComplete(_ ⇒ getButton.disabled = false)
      }
    }

    getButton.onclick = mouseEvent ⇒ {
      getAction
    }

    getInput.onkeypress = keyboardEvent ⇒ {
      if (keyboardEvent.charCode == 13) {
        getAction
      }
    }
  }
}
