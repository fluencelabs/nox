package fluence.node.grpc

import io.grpc.{ClientCall, Metadata}

import scala.concurrent.Promise

class ProxyListener[T](
  onMessagePr: Promise[T],
  onHeadersPr: Promise[Metadata],
  onClosePr: Promise[(io.grpc.Status, Metadata)]
) extends ClientCall.Listener[T] with slogging.LazyLogging {
  override def onHeaders(headers: Metadata): Unit = {
    super.onHeaders(headers)
    onHeadersPr.trySuccess(headers)
    logger.error("HEADERS === " + headers)
  }

  override def onClose(status: io.grpc.Status, trailers: Metadata): Unit = {
    logger.error("ON CLOSE === " + status + "   " + trailers)
    onClosePr.trySuccess((status, trailers))
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
