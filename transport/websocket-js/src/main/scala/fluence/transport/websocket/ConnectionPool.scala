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

import fluence.codec
import fluence.codec.PureCodec
import fluence.proxy.grpc.WebsocketMessage
import fluence.transport.websocket.WebsocketPipe.WebsocketClient
import monix.execution.Scheduler
import monix.reactive.Observable

import scala.concurrent.duration._
import scala.scalajs.js.Date

class ConnectionPool(timeout: Date) {

  private var connections: Map[String, WebsocketClient[WebsocketMessage]] = Map.empty

  def getOrCreateConnection(url: String, builder: String ⇒ WebsocketT)(
    implicit scheduler: Scheduler
  ): WebsocketClient[WebsocketMessage] = {
    connections.getOrElse(
      url, {
        val pipe = WebsocketPipe(url, builder)
        val client = WebsocketPipe.binaryClient(pipe)

        val websocketMessageCodec: codec.PureCodec[WebsocketMessage, Array[Byte]] =
          PureCodec.build[WebsocketMessage, Array[Byte]](
            (m: WebsocketMessage) ⇒ m.toByteArray,
            (arr: Array[Byte]) ⇒ WebsocketMessage.parseFrom(arr)
          )

        val subscription =
          Observable.timerRepeated[WebsocketFrame](5.seconds, 5.seconds, CheckTimeFrame).subscribe(pipe.input)

        val ws =
          client.xmap[WebsocketMessage, WebsocketMessage](websocketMessageCodec.direct, websocketMessageCodec.inverse)

        client.statusOutput.collect {
          case WebsocketLastUsage(time) ⇒
            val now = new Date()
            if ((now.getTime() - new Date(time).getTime()) > timeout.getTime())
              pipe.input.onNext(CloseFrame("Usage timeout."))
          case WebsocketClosed ⇒
            subscription.cancel()
            connections = connections - url
        }.subscribe()

        connections = connections.updated(url, ws)
        ws
      }
    )
  }
}
