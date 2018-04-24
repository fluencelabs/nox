package fluence.transport.websocket

import monix.execution.Ack
import monix.execution.Ack.{Continue, Stop}
import monix.reactive.Observer
import org.scalajs.dom.WebSocket

import scala.scalajs.js.JSConverters._
import scala.scalajs.js.typedarray.TypedArrayBufferOps._
import scala.concurrent.{ExecutionContext, Future}
import scala.scalajs.js.typedarray.{Int8Array, TypedArrayBuffer}

class WebsocketObserver private[websocket] (webSocket: WebSocket)(implicit context: ExecutionContext)
    extends Observer[WebsocketFrame] with slogging.LazyLogging {
  override def onNext(elem: WebsocketFrame): Future[Ack] = {
    Future {
      elem match {
        case Binary(data) ⇒
          val arr = new Int8Array(data.toArray.toJSArray)
          val buffer = TypedArrayBuffer.wrap(arr).arrayBuffer()
          webSocket.send(buffer)
        case Text(data) ⇒
          webSocket.send(data)
      }
    }.map(_ ⇒ Continue).recover {
      case e: Throwable ⇒
        logger.error("Unsupported exception", e)
        Stop
    }
  }

  override def onError(ex: Throwable): Unit = ex.printStackTrace()

  override def onComplete(): Unit = ()
}
