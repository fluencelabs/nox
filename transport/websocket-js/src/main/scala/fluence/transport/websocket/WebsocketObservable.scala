package fluence.transport.websocket

import monix.execution.{Ack, Cancelable}
import monix.reactive.observers.{CacheUntilConnectSubscriber, Subscriber}
import monix.reactive.{Observable, Observer, OverflowStrategy}
import org.scalajs.dom._
import scodec.bits.ByteVector

import scala.concurrent.Future
import scala.scalajs.js.typedarray.{ArrayBuffer, TypedArrayBuffer}
import scala.util.control.{NoStackTrace, NonFatal}

/**
 * Ouput of websocket.
 * @param observer a subscriber who will cache messages until the WebSocket connects
 */
final class WebsocketObservable(webSocket: WebSocket, observer: CacheUntilConnectSubscriber[_])
    extends Observable[WebsocketFrame] with slogging.LazyLogging {
  self ⇒

  //TODO control overflow strategy
  private val overflow: OverflowStrategy.Synchronous[Nothing] = OverflowStrategy.Unbounded

  private def arrayToByteVector(buf: ArrayBuffer): ByteVector = {
    val typed = TypedArrayBuffer.wrap(buf)
    ByteVector(typed)
  }

  private val channel: Observable[WebsocketFrame] =
    Observable.create[WebsocketFrame](overflow) { subscriber ⇒
      try {
        logger.info(s"Connecting to ${webSocket.url}")

        webSocket.onopen = (event: Event) ⇒ {
          observer.connect()
        }
        webSocket.onerror = (event: ErrorEvent) ⇒ {
          subscriber.onError(WebsocketObservable.WebsocketException(event.message))
        }
        webSocket.onclose = (event: CloseEvent) ⇒ {
          subscriber.onComplete()
        }
        webSocket.onmessage = (event: MessageEvent) ⇒ {
          event.data match {
            case s: String ⇒
              subscriber.onNext(Text(s))
            case buf: ArrayBuffer ⇒
              subscriber.onNext(Binary(arrayToByteVector(buf)))
            case b: Blob ⇒
              //we get binary data in websockets usually in blobs
              val fReader = new FileReader()
              fReader.onload = (event: UIEvent) ⇒ {
                val buf = fReader.result.asInstanceOf[ArrayBuffer]
                val bv = arrayToByteVector(buf)
                subscriber.onNext(Binary(bv))
              }
              fReader.onerror = (event: Event) ⇒ {
                logger.error(s"Unexpected error on reading binary frame from websocket ${webSocket.url}")
                subscriber.onError(WebsocketObservable.WebsocketException("Unexpected error on reading binary frame."))
              }

              fReader.readAsArrayBuffer(b)
          }

        }

        Cancelable(() ⇒ {
          logger.info(s"Closing connection to ${webSocket.url}")
          if (webSocket != null && webSocket.readyState <= 1)
            try webSocket.close()
            catch {
              case _: Throwable ⇒ ()
            }
          observer.onComplete()
        })
      } catch {
        case NonFatal(ex) ⇒
          subscriber.onError(ex)
          Cancelable.empty
      }
    }

  override def unsafeSubscribeFn(subscriber: Subscriber[WebsocketFrame]): Cancelable = {
    import subscriber.scheduler

    channel.unsafeSubscribeFn(new Observer[WebsocketFrame] {
      def onNext(elem: WebsocketFrame): Future[Ack] =
        subscriber.onNext(elem)

      //TODO try to reconnect on error and on complete
      def onError(ex: Throwable): Unit = {
        scheduler.reportFailure(ex)
      }

      def onComplete(): Unit = {}
    })
  }
}

object WebsocketObservable {

  case class WebsocketException(errorMessage: String, causedBy: Option[Throwable] = None)
      extends Throwable(errorMessage) with NoStackTrace {

    override def getCause: Throwable = causedBy getOrElse super.getCause
  }
}
