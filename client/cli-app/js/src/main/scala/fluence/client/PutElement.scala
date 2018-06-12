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

object PutElement extends slogging.LazyLogging {

  /**
   * Add element with `put` logic.
   *
   * @param action Action, that will be processed on button click or by pressing `enter` key
   * @param resultField Field, that will be show the result of action.
   */
  def addPutElement(action: (String, String) ⇒ Task[Option[String]], resultField: Input)(
    implicit scheduler: Scheduler
  ): Unit = {
    val putButton = document.getElementById("put-submit").asInstanceOf[Button]
    val putKeyInput = document.getElementById("put-key-input").asInstanceOf[Input]
    val putValueInput = document.getElementById("put-value-input").asInstanceOf[Input]

    def putAction = {
      if (!putButton.disabled) {
        putButton.disabled = true
        val key = putKeyInput.value
        val value = putValueInput.value
        logger.info(s"Put key: $key and value: $value")
        val t = for {
          res ← action(key, value).map(Utils.prettyResult)
        } yield {
          val printResult = s"Put operation success. Old value: $res"
          logger.info(printResult)
          resultField.value = printResult
          putValueInput.value = ""
          putKeyInput.value = ""
        }
        t.runAsync.onComplete(_ ⇒ putButton.disabled = false)
      }
    }

    putButton.onclick = mouseEvent ⇒ {
      putAction
    }

    putValueInput.onkeypress = keyboardEvent ⇒ {
      if (keyboardEvent.charCode == 13) {
        putAction
      }
    }
  }
}
