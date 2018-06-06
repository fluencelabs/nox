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
