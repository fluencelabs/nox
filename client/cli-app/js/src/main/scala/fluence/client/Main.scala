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

import monix.execution.Scheduler.Implicits.global
import org.scalajs.dom.document
import org.scalajs.dom.html.TextArea
import slogging.{LogLevel, LoggerConfig}

import scala.language.higherKinds
import scala.scalajs.js.annotation.JSExportTopLevel

/**
 *
 * This is class for tests only, will be deleted after implementation of browser client.
 *
 */
@JSExportTopLevel("MainInterface")
object Main extends slogging.LazyLogging {

  def initLogging(): Unit = {
    val textArea = document.createElement("textarea").asInstanceOf[TextArea]
    textArea.readOnly = true
    textArea.cols = 200
    textArea.rows = 40
    document.body.appendChild(textArea)

    LoggerConfig.factory = new TextAreaWithConsoleLoggerFactory(textArea, 100)
    LoggerConfig.level = LogLevel.INFO
  }

  def buildInterface(): Unit = {

    initLogging()

    val r = for {
      dataset ← NaiveDataset.createNewDataset()
      lastResultElement = LastResult.addLastResultElement(document.body)
      _ = GetElement.addGetElement(document.body, dataset.get, lastResultElement)
      _ = PutElement.addPutElement(document.body, dataset.put, lastResultElement)
    } yield {
      logger.info("Initialization finished.")
    }

    r.attempt.unsafeToFuture()
  }

  buildInterface()
}
