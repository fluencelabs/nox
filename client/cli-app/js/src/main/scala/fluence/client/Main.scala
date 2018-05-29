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
import fluence.client.core.FluenceClient
import fluence.client.grpc.ClientWebsocketServices
import fluence.codec
import fluence.codec.PureCodec
import fluence.crypto.aes.{AesConfig, AesCrypt}
import fluence.crypto.ecdsa.Ecdsa
import fluence.crypto.hash.JsCryptoHasher
import fluence.crypto.signature.SignAlgo
import fluence.crypto.{Crypto, KeyPair}
import fluence.kad.KademliaConf
import fluence.kad.protocol.Contact
import fluence.proxy.grpc.WebsocketMessage
import fluence.transport.websocket.{ConnectionPool, Websocket}
import monix.eval.Task
import monix.execution.Scheduler.Implicits.global
import org.scalajs.dom.document
import org.scalajs.dom.html.{Button, Input, TextArea}
import slogging.{LogLevel, LoggerConfig}

import scala.concurrent.duration._
import scala.language.higherKinds
import scala.scalajs.js.Date
import scala.scalajs.js.annotation.{JSExport, JSExportTopLevel}

/**
 *
 * This is class for tests only, will be deleted after implementation of browser client.
 *
 */
@JSExportTopLevel("MainD")
object Main extends slogging.LazyLogging {

  @JSExport
  def logic(logArea: TextArea): Unit = {

    LoggerConfig.factory = new TextAreaWithConsoleLoggerFactory(logArea, 100)
    LoggerConfig.level = LogLevel.INFO

    val algo: SignAlgo = Ecdsa.signAlgo
    import algo.checker

    logger.info("")

    val hasher: Crypto.Hasher[Array[Byte], Array[Byte]] = JsCryptoHasher.Sha256

    val seedContact = Contact.readB64seed.unsafe(
      "eyJwayI6IkE1dXlwajBkcXBZdDYtcWNvMmhMME14Y2Flbm4xUHF2X1FmTXNrMG1uaFpDIiwicHYiOjB9.eyJhIjoiMTI3LjAuMC4xIiwiZ3AiOjExMDIyLCJnaCI6IjAwMDAwMDAwMDAwMDAwMDAwMDAwIiwid3AiOjgwOTJ9.MEUCIE94PSeplPfqkoDGU22ckOxQv2gpb5TN2E2MbSQIvTr7AiEAyLrGLC4RnsqlBuAo-AmegXeibYpjTtjteZHRAZrHdf8="
    )

    val kadConfig = KademliaConf(3, 3, 1, 5.seconds)

    val timeout = {
      val date = new Date(0)
      date.setSeconds(3)
      date
    }

    implicit val websocketMessageCodec: codec.PureCodec[WebsocketMessage, Array[Byte]] =
      PureCodec.build[WebsocketMessage, Array[Byte]](
        (m: WebsocketMessage) ⇒ m.toByteArray,
        (arr: Array[Byte]) ⇒ WebsocketMessage.parseFrom(arr)
      )

    val connectionPool = ConnectionPool[WebsocketMessage](timeout, 1.second, builder = Websocket.builder)
    val clientWebsocketServices = new ClientWebsocketServices(connectionPool)

    val client = clientWebsocketServices.build[Task]

    val clIO = FluenceClient.build(Seq(seedContact), algo, hasher, kadConfig, client andThen (_.get))

    def cryptoMethods(
      secretKey: KeyPair.Secret
    ): (Crypto.Cipher[String], Crypto.Cipher[String]) = {
      val aesConfig = AesConfig(
        50
      )
      (
        AesCrypt.forString(secretKey.value, withIV = false, aesConfig),
        AesCrypt.forString(secretKey.value, withIV = false, aesConfig)
      )
    }

    val r = for {
      cl ← clIO
      newkey ← algo.generateKeyPair.runF[IO](None)
      crypts = cryptoMethods(newkey.secretKey)
      (keyCrypt, valueCrypt) = crypts
      dataset ← cl.createNewContract(newkey, 2, keyCrypt, valueCrypt).toIO
      _ = addGetForm(dataset.get)
      _ = addPutForm(dataset.put)
    } yield {
      logger.info("Initialization finished.")
    }

    r.attempt.unsafeToFuture()
  }

  def main(args: Array[String]): Unit = {
    println("start main")
//    logic()
  }

  def addGetForm(action: String ⇒ Task[Option[String]]): Unit = {
    val getInput = document.createElement("input").asInstanceOf[Input]
    getInput.`type` = "input"
    getInput.name = "put"

    val getButton = document.createElement("input").asInstanceOf[Input]
    getButton.`type` = "submit"
    getButton.value = "Get"

    document.body.appendChild(getInput)
    document.body.appendChild(getButton)

    getButton.onclick = mouseEvent ⇒ {
      getButton.disabled = true
      val key = getInput.value
      logger.info("\n")
      logger.info("==========GET START==========")
      logger.info(s"Get key: $key")
      val t = for {
        res ← action(key)
      } yield {
        logger.info(s"Get operation success. Value: ${prettyResult(res)}")
        getInput.value = ""
        logger.info("==========GET END==========\n")
      }
      t.runAsync.onComplete(_ ⇒ getButton.disabled = false)
    }
  }

  def prettyResult(result: Option[String]) = s"`${result.getOrElse("null")}`"

  def addPutForm(action: (String, String) ⇒ Task[Option[String]]): Unit = {

    val putKeyInput = document.createElement("input").asInstanceOf[Input]
    putKeyInput.`type` = "text"
    putKeyInput.name = "putKey"
    putKeyInput.value = ""

    val putValueInput = document.createElement("input").asInstanceOf[Input]
    putValueInput.`type` = "text"
    putValueInput.name = "putValue"
    putValueInput.value = ""

    val putButton = document.createElement("input").asInstanceOf[Button]
    putButton.`type` = "submit"
    putButton.value = "Put"

    document.body.appendChild(putKeyInput)
    document.body.appendChild(putValueInput)
    document.body.appendChild(putButton)

    putButton.onclick = mouseEvent ⇒ {
      putButton.disabled = true
      val key = putKeyInput.value
      val value = putValueInput.value
      logger.info("\n")
      logger.info("==========PUT START==========")
      logger.info(s"Put key: $key and value: $value")
      val t = for {
        res ← action(key, value)
      } yield {
        logger.info(s"Put operation success. Old value: ${prettyResult(res)}")
        putValueInput.value = ""
        putKeyInput.value = ""
        logger.info("==========PUT END==========\n")
      }
      t.runAsync.onComplete(_ ⇒ putButton.disabled = false)
    }
  }

  def addTextArea() = {
    val textArea = document.createElement("textarea").asInstanceOf[TextArea]
    textArea.value = "PUSTO"
    textArea.readOnly = true
    textArea.cols = 200
    textArea.rows = 40

    document.body.appendChild(textArea)

    textArea
  }

  val textArea = addTextArea()

  println("CREATE TEXT AREA")

  logic(textArea)
}
