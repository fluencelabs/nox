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
    logger.debug("onClose callback: " + status + "   " + trailers)
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
