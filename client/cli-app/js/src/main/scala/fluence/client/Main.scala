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

import fluence.crypto.KeyPair
import fluence.crypto.ecdsa.Ecdsa
import fluence.crypto.keystore.KeyStore
import fluence.crypto.signature.SignAlgo
import fluence.kad.protocol.Contact
import monix.eval.Task
import monix.execution.Scheduler.Implicits.global
import org.scalajs.dom.document
import org.scalajs.dom.html.{Div, TextArea}
import org.scalajs.dom.raw.HTMLElement
import slogging.{LogLevel, LoggerConfig}

import scala.language.higherKinds
import scala.scalajs.js.{Any, Array, JSON}
import scala.scalajs.js.annotation.JSExportTopLevel

/**
 *
 * This is class for tests only, will be deleted after implementation of browser client.
 *
 */
@JSExportTopLevel("MainInterface")
object Main extends slogging.LazyLogging {

  def main(args: Array[String]) = {}

  def initLogging(): Unit = {
    val textArea = document.createElement("textarea").asInstanceOf[TextArea]
    textArea.readOnly = true
    textArea.cols = 160
    textArea.rows = 30
    document.body.appendChild(textArea)

    LoggerConfig.factory = new TextAreaWithConsoleLoggerFactory(textArea, 100)
    LoggerConfig.level = LogLevel.INFO
  }

  def mainWorkAction(keysJson: Option[String], keysElement: HTMLElement, algo: SignAlgo) = {

    import algo.checker

    val seedContact = Contact.readB64seed.unsafe(
      "eyJwayI6IkE5ZmZaWS1FbG5aSlNCWEJBMno4Q2FpWTNLT051Y3doTkdfY0FmRVNNU3liIiwicHYiOjB9.eyJhIjoiMTI3LjAuMC4xIiwiZ3AiOjExMDIxLCJnaCI6IjAwMDAwMDAwMDAwMDAwMDAwMDAwIiwid3AiOjgwOTF9.MEUCIAu0lDokN_cMOZzgVXzCdPNPhhFVWEBkhP5vbv_EGUL3AiEA73MbbvNAANW6BTin-jho9Dsv42X2iqtgv-s5vpgGdQo="
    )

    val keyPair = algo.generateKeyPair.unsafe(None)

    for {
      dataset ← NaiveDataset.createNewDataset(keysJson, algo, seedContact, keyPair)
      lastResultElement = LastResult.addLastResultElement(document.body)
      _ = GetElement.addGetElement(document.body, dataset.get, lastResultElement)
      _ = PutElement.addPutElement(document.body, dataset.put, lastResultElement)
    } yield {
      logger.info("Initialization finished.")
    }
  }

  def buildInterface(): Unit = {

    initLogging()

    val algo: SignAlgo = Ecdsa.signAlgo
    import KeyStore._

    val generateAction: Task[String] =
      for {
        kp ← algo.generateKeyPair.runF[Task](None)
        kpStr ← keyPairJsonStringCodec.direct.runF[Task](kp)
      } yield JSON.stringify(JSON.parse(kpStr), null: Array[Any], 2)

    def validateAction(keyPair: String): Task[Either[String, KeyPair]] =
      keyPairJsonStringCodec
        .inverse[Task](keyPair)
        .leftMap(_.message)
        .value

    def submitAction(keyPair: KeyPair): Task[Unit] = {
      for {
        _ ← Task.unit
      } yield {}
    }

    KeysElement.addKeysElement(document.body, generateAction, validateAction, submitAction)

  }

  buildInterface()
}
