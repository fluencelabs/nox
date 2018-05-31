package fluence.client

import fluence.crypto.KeyPair
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
    validatingAction: String ⇒ Task[Either[String, KeyPair]],
    submitAction: KeyPair ⇒ Task[Unit],
    text: TextArea
  )(implicit scheduler: Scheduler) = {
    val submitButton = document.createElement("input").asInstanceOf[Button]
    submitButton.`type` = "submit"
    submitButton.value = "Submit"

    submitButton.onclick = me ⇒ {

      val keyPairStr = text.value

      val onError = for {
        _ ← Task.unit
        _ = println("ALL BAD")
      } yield {}

      val t = for {
        validate ← validatingAction(keyPairStr)
        _ ← validate match {
          case Left(str) ⇒ onError
          case Right(kp) ⇒ submitAction(kp)
        }
      } yield {}
      t.runAsync
    }

    submitButton
  }

  def addKeysElement(
    el: HTMLElement,
    generateKeyAction: Task[String],
    validatingAction: String ⇒ Task[Either[String, KeyPair]],
    submitAction: KeyPair ⇒ Task[Unit]
  )(implicit scheduler: Scheduler) = {
    val div = document.createElement("div").asInstanceOf[Div]

    div.innerHTML += "Add your keys here or generate a new one and save it:"
    div.appendChild(document.createElement("br"))

    val textArea = document.createElement("textarea").asInstanceOf[TextArea]
    textArea.cols = 100
    textArea.rows = 20
    div.appendChild(textArea)

    div.appendChild(document.createElement("br"))

    val genAction = generateKeyAction.map(key ⇒ textArea.value = key)
    val generatedButton = generateButton(genAction)
    div.appendChild(generatedButton)

    div.appendChild(document.createElement("br"))

    val submitedButton = submitButton(validatingAction, submitAction, textArea)
    div.appendChild(submitedButton)

    div.appendChild(document.createElement("br"))

    el.appendChild(div)

  }
}
