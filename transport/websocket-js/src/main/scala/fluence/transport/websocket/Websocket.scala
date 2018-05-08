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
