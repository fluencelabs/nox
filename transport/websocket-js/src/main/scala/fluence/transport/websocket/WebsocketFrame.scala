package fluence.transport.websocket

import scodec.bits.ByteVector

sealed trait WebsocketFrame
final case class Binary(data: ByteVector) extends WebsocketFrame
final case class Text(data: String) extends WebsocketFrame
