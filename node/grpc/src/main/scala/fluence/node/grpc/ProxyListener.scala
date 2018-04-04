package fluence.node.grpc

import io.grpc.{ClientCall, Metadata, Status}

import scala.concurrent.Promise

class ProxyListener[T](
  onMessagePr: Promise[T],
  onHeadersPr: Option[Promise[Metadata]] = None,
  onClosePr: Option[Promise[(io.grpc.Status, Metadata)]] = None
) extends ClientCall.Listener[T] with slogging.LazyLogging {
  override def onHeaders(headers: Metadata): Unit = {
    super.onHeaders(headers)
    onHeadersPr.foreach(_.trySuccess(headers))
    logger.error("HEADERS === " + headers)
  }

  override def onClose(status: io.grpc.Status, trailers: Metadata): Unit = {
    logger.error("ON CLOSE === " + status + "   " + trailers)
    onClosePr.foreach(_.trySuccess((status, trailers)))
    super.onClose(status, trailers)
  }

  override def onMessage(message: T): Unit = {
    logger.error("ON MESSAGE === " + message)
    onMessagePr.trySuccess(message)
    super.onMessage(message)
  }

  override def onReady(): Unit = {
    logger.error("ON READY === ")
    super.onReady()
  }
}
