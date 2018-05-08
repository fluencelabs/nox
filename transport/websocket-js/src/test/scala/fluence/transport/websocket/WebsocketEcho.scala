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

package fluence.transport.websocket

import java.nio.ByteBuffer

import org.scalajs.dom._
import org.scalajs.dom.raw.Blob

import scala.scalajs.js
import scala.scalajs.js.typedarray.{ArrayBuffer, Int8Array, TypedArrayBuffer}
import scala.scalajs.js.JSConverters._

/**
 * Mock websocket that echo for each messages except `errorOnceBytes` and `closeWebsocketBytes` control messages.
 * @param url
 */
case class WebsocketEcho(
  url: String
) extends WebsocketT {

  def newEvent(data: js.Any): MessageEvent = {
    val me: MessageEvent = document.createEvent("MessageEvent").asInstanceOf[MessageEvent]
    me.initMessageEvent("MessageEvent", true, false, data, "ws://localhost", "", org.scalajs.dom.window)
    me
  }

  def errorEvent(message: String): ErrorEvent = {
    val event = js.Dynamic
      .literal(
        message = message,
        lineno = 0
      )
      .asInstanceOf[ErrorEvent]
    event
  }

  def closeEvent(code: Int, message: String): CloseEvent = {
    val event = js.Dynamic
      .literal(
        reason = message,
        code = code
      )
      .asInstanceOf[CloseEvent]
    event
  }

  private val errorOnceBytes: ByteBuffer =
    TypedArrayBuffer.wrap(new Int8Array(WebsocketEcho.errorOnceOnSendMessage.toJSArray))
  private val closeWebsocketBytes: ByteBuffer =
    TypedArrayBuffer.wrap(new Int8Array(WebsocketEcho.closeWebsocketMessage.toJSArray))
  private val errorWebsocketBytes: ByteBuffer =
    TypedArrayBuffer.wrap(new Int8Array(WebsocketEcho.errorWebsocketMessage.toJSArray))

  var onopen: Event ⇒ Unit = null
  var onmessage: MessageEvent ⇒ Unit = null
  var onerror: ErrorEvent ⇒ Unit = null
  var onclose: CloseEvent ⇒ Unit = null

  override def send(data: String): Unit = onmessage(newEvent(data))

  override def send(data: Blob): Unit = onmessage(newEvent(data))

  override def send(data: ArrayBuffer): Unit = {
    if (!WebsocketEcho.errored && errorOnceBytes == TypedArrayBuffer.wrap(data)) {
      WebsocketEcho.errored = true
      throw new RuntimeException("Some error")
    } else if (closeWebsocketBytes == TypedArrayBuffer.wrap(data))
      close()
    else if (errorWebsocketBytes == TypedArrayBuffer.wrap(data)) {
      onerror(errorEvent("something wrong"))
    } else {
      WebsocketEcho.errored = false
      onmessage(newEvent(data))
    }
  }

  override def close(code: Int, reason: String): Unit = onclose(closeEvent(code, reason))

  override def close(): Unit = onclose(closeEvent(1000, ""))

  override def setOnopen(onopen: Event ⇒ Unit): Unit = {
    this.onopen = onopen
    onopen(newEvent(""))
  }
  override def setOnmessage(onmessage: MessageEvent ⇒ Unit): Unit = this.onmessage = onmessage
  override def setOnerror(onerror: ErrorEvent ⇒ Unit): Unit = this.onerror = onerror
  override def setOnclose(onclose: CloseEvent ⇒ Unit): Unit = this.onclose = onclose

  override def readyState: Int = 1
}

object WebsocketEcho {
  val errorOnceOnSendMessage: Array[Byte] = Array[Byte](1, 1, 1, 1, 1, 1, 1)
  val closeWebsocketMessage: Array[Byte] = Array[Byte](2, 2, 2, 2, 2, 2, 2)
  val errorWebsocketMessage: Array[Byte] = Array[Byte](3, 3, 3, 3, 3, 3, 3)
  var errored = false
}
