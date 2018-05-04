package fluence.transport.websocket

import java.nio.ByteBuffer

import org.scalajs.dom._
import org.scalajs.dom.raw.Blob

import scala.scalajs.js
import scala.scalajs.js.typedarray.{ArrayBuffer, Int8Array, TypedArrayBuffer}
import scala.scalajs.js.JSConverters._

case class WebsocketEcho(
  url: String
) extends WebsocketT {

  def newEvent(data: js.Any): MessageEvent = {
    val me: MessageEvent = document.createEvent("MessageEvent").asInstanceOf[MessageEvent]
    me.initMessageEvent("MessageEvent", true, false, data, "ws://localhost", "", org.scalajs.dom.window)
    me
  }

  def errorEvent(message: String): ErrorEvent = {
    val me: ErrorEvent = document.createEvent("ErrorEvent").asInstanceOf[ErrorEvent]
    me.initErrorEvent("ErrorEvent", true, false, message, "", 0)
    me
  }

  def closeEvent(code: Int, message: String): CloseEvent = {
    val me: CloseEvent = document.createEvent("CloseEvent").asInstanceOf[CloseEvent]
    me.initCloseEvent("CloseEvent", true, false, true, code, message)
    me
  }

  private val errorOnceBytes: ByteBuffer = TypedArrayBuffer.wrap(new Int8Array(WebsocketEcho.errorOnceArray.toJSArray))
  var errored = false

  var onopen: Event ⇒ Unit = null
  var onmessage: MessageEvent ⇒ Unit = null
  var onerror: ErrorEvent ⇒ Unit = null
  var onclose: CloseEvent ⇒ Unit = null

  override def send(data: String): Unit = onmessage(newEvent(data))

  override def send(data: Blob): Unit = onmessage(newEvent(data))

  override def send(data: ArrayBuffer): Unit = {
    if (!errored && errorOnceBytes == TypedArrayBuffer.wrap(data)) {
      errored = true
      onerror(errorEvent("BOOM"))
    } else {
      onmessage(newEvent(data))
    }
  }

  override def close(code: Int, reason: String): Unit = onclose(closeEvent(code, reason))

  override def close(): Unit = onclose(closeEvent(0, ""))

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
  val errorOnceArray: Array[Byte] = Array[Byte](1, 1, 1, 1, 1, 1, 1)
}
