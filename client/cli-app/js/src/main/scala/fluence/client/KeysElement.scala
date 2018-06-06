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
import org.scalajs.dom.html.{Button, Div, TextArea}
import org.scalajs.dom.raw.HTMLElement

object KeysElement {

  private def generateButton(generateKeyAction: Task[Unit])(implicit scheduler: Scheduler) = {
    val generateButton = document.createElement("input").asInstanceOf[Button]
    generateButton.`type` = "submit"
    generateButton.value = "Generate"

    generateButton.onclick = mouseEvent ⇒ {
      generateKeyAction.runAsync
    }

    generateButton
  }

  private def submitButton(
    submitAction: String ⇒ Task[Unit],
    text: TextArea
  )(implicit scheduler: Scheduler) = {
    val submitButton = document.createElement("input").asInstanceOf[Button]
    submitButton.`type` = "submit"
    submitButton.value = "Submit"

    submitButton.onclick = me ⇒ {

      val keyPairStr = text.value

      submitAction(keyPairStr).runAsync
    }

    submitButton
  }

  def addKeysElement(
    el: HTMLElement,
    generateKeyAction: Task[String],
    submitAction: String ⇒ Task[Unit]
  )(implicit scheduler: Scheduler): Div = {
    val div = document.createElement("div").asInstanceOf[Div]

    div.innerHTML += "Add your keys here or generate a new one and save it:"
    div.appendChild(document.createElement("br"))

    val textArea = document.createElement("textarea").asInstanceOf[TextArea]
    textArea.cols = 60
    textArea.rows = 8
    div.appendChild(textArea)

    div.appendChild(document.createElement("br"))

    val genAction = generateKeyAction.map(key ⇒ textArea.value = key)
    val generatedButton = generateButton(genAction)
    div.appendChild(generatedButton)

    div.appendChild(document.createElement("br"))

    val submitedButton = submitButton(submitAction, textArea)
    div.appendChild(submitedButton)

    div.appendChild(document.createElement("br"))

    el.appendChild(div)

    div
  }
}
