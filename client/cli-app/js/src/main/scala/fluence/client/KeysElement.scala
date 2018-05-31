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
