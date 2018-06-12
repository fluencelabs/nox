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
import org.scalajs.dom.html.{Button, TextArea}

object KeysElement {

  def addKeysElement(
    generateKeyAction: Task[String],
    submitAction: String ⇒ Task[Unit]
  )(implicit scheduler: Scheduler): Unit = {
    val genButton = document.getElementById("key-generator-gen").asInstanceOf[Button]
    genButton.onclick = mouseEvent ⇒ {
      generateKeyAction.map { keys ⇒
        document.getElementById("key-generator").asInstanceOf[TextArea].value = keys
      }.runAsync
    }

    val submitButton = document.getElementById("key-generator-submit").asInstanceOf[Button]
    submitButton.onclick = me ⇒ {

      val keyPairStr = document.getElementById("key-generator").asInstanceOf[TextArea].value

      submitAction(keyPairStr).runAsync
    }
  }
}
