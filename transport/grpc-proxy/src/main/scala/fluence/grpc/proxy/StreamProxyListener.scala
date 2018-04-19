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

package fluence.grpc.proxy

import io.grpc.{ClientCall, Metadata}
import monix.reactive.Observer

/**
 * A listener who pulls messages to the observer and closes the observer when the call is closed.
 *
 * @tparam T Type of the message.
 */
class StreamProxyListener[T](
  obs: Observer[T]
) extends ClientCall.Listener[T] with slogging.LazyLogging {

  override def onHeaders(headers: Metadata): Unit = {
    super.onHeaders(headers)
    logger.debug("STREAM: onHeaders callback: " + headers)

  }

  override def onClose(status: io.grpc.Status, trailers: Metadata): Unit = {
    super.onClose(status, trailers)
    obs.onComplete()
    logger.debug(s"STREAM: onClose callback: Status: $status, trailers: $trailers")

  }

  override def onMessage(message: T): Unit = {
    super.onMessage(message)
    obs.onNext(message)
    logger.debug("STREAM: onMessage callback: " + message)
  }

  override def onReady(): Unit = {
    super.onReady()
    logger.debug("STREAM: onReady callback.")
  }
}
