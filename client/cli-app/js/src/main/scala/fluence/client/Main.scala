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

import cats.effect.IO
import fluence.crypto.KeyPair
import fluence.crypto.ecdsa.Ecdsa
import fluence.crypto.keystore.KeyStore
import fluence.crypto.signature.SignAlgo
import fluence.kad.protocol.Contact
import monix.eval.Task
import monix.execution.Scheduler.Implicits.global
import org.scalajs.dom.document
import org.scalajs.dom.html.{Div, Input, TextArea}
import slogging.{LogLevel, LoggerConfig}

import scala.language.higherKinds
import scala.scalajs.js.annotation.{JSExport, JSExportTopLevel}
import scala.scalajs.js.{Any, JSON}

/**
 *
 * This is class for tests only, will be deleted after implementation of browser client.
 *
 */
@JSExportTopLevel("MainInterface")
object Main extends slogging.LazyLogging {

  private def initLogging(): Unit = {

    val textArea = document.getElementById("logger").asInstanceOf[TextArea]

    LoggerConfig.factory = new TextAreaWithConsoleLoggerFactory(textArea, 100)
    LoggerConfig.level = LogLevel.INFO
  }

  private def mainWorkAction(seed: String, keysPair: KeyPair, algo: SignAlgo): IO[Unit] = {

    import algo.checker

    val seedContact = Contact.readB64seed.unsafe(seed)

    for {
      dataset ← NaiveDataset.createNewDataset(algo, seedContact, keysPair)
      lastResultElement = document.getElementById("last-result").asInstanceOf[Input]
      _ = GetElement.addGetElement(dataset.get, lastResultElement)
      _ = PutElement.addPutElement(dataset.put, lastResultElement)
      _ = RangeElement.addrangeElement(dataset.range, lastResultElement)
    } yield {
      logger.info("Initialization finished.")
    }
  }

  @JSExport
  def buildInterface(seed: String): Unit = {

    initLogging()

    val algo: SignAlgo = Ecdsa.signAlgo
    import KeyStore._

    def generateAction: Task[String] = Task.defer {
      for {
        kp ← algo.generateKeyPair.runF[Task](None)
        kpStr ← keyPairJsonStringCodec.direct.runF[Task](kp)

      } yield {
        JSON.stringify(JSON.parse(kpStr), null: scala.scalajs.js.Array[Any], 2)
      }
    }

    def validateAction(keyPair: String): Task[Either[String, KeyPair]] =
      keyPairJsonStringCodec
        .inverse[Task](keyPair)
        .leftMap(_.message)
        .value

    def submitAction(keyPairStr: String): Task[Unit] = {
      for {
        validate ← validateAction(keyPairStr)
        _ ← validate match {
          case Left(err) ⇒
            logger.info(s"Key is not correct. Error: $err")
            Task.unit
          case Right(kp) ⇒
            val keysEl = document.getElementById("keysEl")
            val parent = keysEl.parentNode
            parent.removeChild(keysEl)

            Task.fromIO(mainWorkAction(seed, kp, algo)).map { _ ⇒
              val mainDiv = document.getElementById("progress-block").asInstanceOf[Div]
              mainDiv.style = "display: block;"
            }
        }
      } yield {}
    }

    KeysElement.addKeysElement(generateAction, submitAction)
  }
}
