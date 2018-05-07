package fluence.transport.websocket

import org.scalajs.dom.raw.Blob
import org.scalajs.dom._

import scala.scalajs.js.typedarray.ArrayBuffer

/**
 * Trait for wrapping websocket.
 */
trait WebsocketT {
  def setOnopen(onopen: Event ⇒ Unit): Unit
  def setOnmessage(onmessage: MessageEvent ⇒ Unit): Unit
  def setOnerror(onerror: ErrorEvent ⇒ Unit): Unit
  def setOnclose(onclose: CloseEvent ⇒ Unit): Unit
  def readyState: Int
  def send(data: String): Unit
  def send(data: Blob): Unit
  def send(data: ArrayBuffer): Unit
  def close(code: Int, reason: String)
  def close()
}

case class Websocket(url: String) extends WebsocketT {

  val websocket = new WebSocket(url)

  override def send(data: String): Unit = websocket.send(data)
  override def send(data: Blob): Unit = websocket.send(data)
  override def send(data: ArrayBuffer): Unit = websocket.send(data)

  override def close(code: Int, reason: String): Unit = websocket.close(code, reason)
  override def close(): Unit = websocket.close()

  override def setOnopen(onopen: Event ⇒ Unit): Unit = websocket.onopen = onopen
  override def setOnmessage(onmessage: MessageEvent ⇒ Unit): Unit = websocket.onmessage = onmessage
  override def setOnerror(onerror: ErrorEvent ⇒ Unit): Unit = websocket.onerror = onerror
  override def setOnclose(onclose: CloseEvent ⇒ Unit): Unit = websocket.onclose = onclose

  override def readyState: Int = websocket.readyState
}
