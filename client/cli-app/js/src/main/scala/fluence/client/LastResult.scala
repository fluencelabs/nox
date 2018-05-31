package fluence.client

import org.scalajs.dom.raw.HTMLElement
import org.scalajs.dom.document
import org.scalajs.dom.html.{Div, Input, Span}

object LastResult {

  /**
   * Element for showing last result of actions.
   */
  def addLastResultElement(el: HTMLElement): Input = {

    val div = document.createElement("div").asInstanceOf[Div]

    val text = document.createElement("input").asInstanceOf[Input]

    text.`type` = "text"
    text.readOnly = true
    text.size = 40

    div.appendChild(document.createElement("br"))
    div.innerHTML += "Last result:"
    div.appendChild(document.createElement("br"))
    div.appendChild(text)
    div.appendChild(document.createElement("br"))

    el.appendChild(div)

    text
  }
}
