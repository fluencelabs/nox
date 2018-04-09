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

import scala.concurrent.Promise

/**
 * Callbacks for receiving metadata, response messages and completion status from the server.
 *
 * @param onMessagePr Promise on message received.
 * @param onHeadersPr Promise on headers received.
 * @param onClosePr Promise on close call received.
 *
 * @tparam T Type of the message.
 */
class ProxyListener[T](
  onMessagePr: Promise[T],
  onHeadersPr: Option[Promise[Metadata]] = None,
  onClosePr: Option[Promise[(io.grpc.Status, Metadata)]] = None
) extends ClientCall.Listener[T] with slogging.LazyLogging {

  override def onHeaders(headers: Metadata): Unit = {
    super.onHeaders(headers)
    logger.debug("onHeaders callback: " + headers)
    onHeadersPr.foreach(_.trySuccess(headers))
  }

  override def onClose(status: io.grpc.Status, trailers: Metadata): Unit = {
    super.onClose(status, trailers)
    logger.debug(s"onClose callback: Status: $status, trailers: $trailers")
    onClosePr.foreach(_.trySuccess((status, trailers)))
  }

  override def onMessage(message: T): Unit = {
    super.onMessage(message)
    logger.debug("onMessage callback: " + message)
    onMessagePr.trySuccess(message)
  }

  override def onReady(): Unit = {
    super.onReady()
    logger.debug("onReady callback.")
  }
}
