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

import fluence.codec.PureCodec
import monix.execution.Scheduler
import monix.reactive.Observable

import scala.concurrent.duration._
import scala.scalajs.js.Date

/**
 * Pool of binary websocket connections, that will be cleaned after timeout if not used.
 *
 * @param timeout Connection will be cleaned after this timeout if not used.
 * @param checkInterval Interval between checking connection.
 * @param builder Websocket creation function.
 * @param inCodec Function from `I` to `Array[Byte]`
 * @param outCodec Function from `Array[Byte]` to `O`
 * @tparam I Type of outgoing messages.
 * @tparam O Type of incoming messages.
 */
class ConnectionPool[I, O](timeout: Date, checkInterval: FiniteDuration = 5.seconds, builder: String ⇒ WebsocketT)(
  implicit inCodec: PureCodec.Func[I, Array[Byte]],
  outCodec: PureCodec.Func[Array[Byte], O]
) {

  private var connections: Map[String, WebsocketPipe[I, O]] = Map.empty

  def getOrCreateConnection(url: String)(
    implicit scheduler: Scheduler
  ): WebsocketPipe[I, O] = {
    connections.getOrElse(
      url, {
        val pipe = WebsocketPipe(url, builder)

        val binaryPipe = WebsocketPipe.binaryClient(pipe)

        val subscription =
          Observable.timerRepeated[WebsocketFrame](checkInterval, checkInterval, CheckTimeFrame).subscribe(pipe.input)

        binaryPipe.statusOutput.collect {
          case WebsocketLastUsage(time) ⇒
            val now = new Date()
            if ((now.getTime() - new Date(time).getTime()) > timeout.getTime())
              pipe.input.onNext(CloseFrame("Usage timeout."))
          case WebsocketClosed ⇒
            subscription.cancel()
            connections = connections - url
        }.subscribe()

        val ws =
          binaryPipe.xmap[I, O](inCodec, outCodec)

        connections = connections.updated(url, ws)
        ws
      }
    )
  }

  /**
   * For tests purpose only.
   */
  def getConnections: Map[String, WebsocketPipe[I, O]] = connections
}

object ConnectionPool {

  /**
   * For a connection pool, if input and output type is the same.
   */
  def apply[T](timeout: Date, checkInterval: FiniteDuration = 5.seconds, builder: String ⇒ WebsocketT)(
    implicit codec: PureCodec[T, Array[Byte]]
  ): ConnectionPool[T, T] = {
    new ConnectionPool[T, T](timeout, checkInterval, builder)(codec.direct, codec.inverse)
  }
}
