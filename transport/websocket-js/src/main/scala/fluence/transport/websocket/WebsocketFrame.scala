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

sealed trait WebsocketFrame extends Any
final case class Binary(data: Array[Byte]) extends AnyVal with WebsocketFrame
final case class Text(data: String) extends AnyVal with WebsocketFrame

sealed trait ControlFrame extends WebsocketFrame
final case class CloseFrame(cause: String) extends ControlFrame
object CheckTimeFrame extends ControlFrame

sealed trait StatusFrame
object WebsocketOnOpen extends StatusFrame
final case class WebsocketOnError(message: String) extends StatusFrame
final case class WebsocketOnClose(code: Int, reason: String) extends StatusFrame
final case class WebsocketLastUsage(time: Double) extends StatusFrame
