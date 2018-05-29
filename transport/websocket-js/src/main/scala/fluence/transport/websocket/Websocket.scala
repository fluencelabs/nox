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

import org.scalajs.dom.raw.Blob
import org.scalajs.dom._

import scala.scalajs.js.typedarray.ArrayBuffer

/**
 * Trait for wrapping websocket. For possibility to mock websocket and test logic.
 */
trait WebsocketT {

  /**
   * An event listener to be called when the WebSocket connection's readyState changes
   * to OPEN; this indicates that the connection is ready to send and receive data. The
   * event is a simple one with the name "open".
   *
   * MDN
   */
  def setOnopen(onopen: Event ⇒ Unit): Unit

  /**
   * An event listener to be called when a message is received from the server. The
   * listener receives a MessageEvent named "message".
   *
   * MDN
   */
  def setOnmessage(onmessage: MessageEvent ⇒ Unit): Unit

  /**
   * An event listener to be called when an error occurs. This is a simple event named
   * "error".
   *
   * MDN
   */
  def setOnerror(onerror: ErrorEvent ⇒ Unit): Unit

  /**
   * An event listener to be called when the WebSocket connection's readyState changes
   * to CLOSED. The listener receives a CloseEvent named "close".
   *
   * MDN
   */
  def setOnclose(onclose: CloseEvent ⇒ Unit): Unit

  /**
   * The current state of the connection; this is one of the Ready state constants. Read
   * only.
   *
   * MDN
   */
  def readyState: Int

  /**
   * Transmits data to the server over the WebSocket connection.
   *
   * MDN
   */
  def send(data: String): Unit
  def send(data: Blob): Unit
  def send(data: ArrayBuffer): Unit

  /**
   * Closes the WebSocket connection or connection attempt, if any. If the connection
   * is already CLOSED, this method does nothing.
   *
   * MDN
   */
  def close(code: Int, reason: String)
  def close()
}

/**
 * Implementation of trait WebsocketT with dom.WebSocket from browser.
 * @param url Address to connect.
 */
case class Websocket(url: String) extends WebsocketT {

  private val websocket = new WebSocket(url)

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

object Websocket {
  def builder: String ⇒ WebsocketT = str ⇒ Websocket(str)
}
