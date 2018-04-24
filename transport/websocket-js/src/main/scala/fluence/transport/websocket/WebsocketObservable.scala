package fluence.transport.websocket

import monix.eval.Task
import monix.execution.{Ack, Cancelable}
import monix.execution.rstreams.Subscription
import monix.reactive.{Observable, Observer, OverflowStrategy}
import monix.reactive.observers.{CacheUntilConnectSubscriber, Subscriber}
import org.scalajs.dom._
import scodec.bits.ByteVector

import scala.concurrent.{Future, Promise}
import scala.concurrent.duration._
import scala.scalajs.js.typedarray.{ArrayBuffer, Int8Array, TypedArrayBuffer}
import scala.util.control.NonFatal

final class WebsocketObservable(webSocket: WebSocket, observer: CacheUntilConnectSubscriber[_])
    extends Observable[WebsocketFrame] {
  self ⇒

  private val overflow: OverflowStrategy.Synchronous[Nothing] = OverflowStrategy.Unbounded

  private def arrayToByteVector(buf: ArrayBuffer): ByteVector = {
    val typed = TypedArrayBuffer.wrap(buf)
    ByteVector(typed)
  }

  private val channel: Observable[WebsocketFrame] =
    Observable.create[WebsocketFrame](overflow) { subscriber ⇒
      def closeConnection(webSocket: WebSocket): Unit = {
        println(s"Closing connection to ${webSocket.url}")
        if (webSocket != null && webSocket.readyState <= 1)
          try webSocket.close()
          catch {
            case _: Throwable ⇒ ()
          }
        observer.onComplete()
      }

      try {
        println(s"Connecting to ${webSocket.url}")

        webSocket.onopen = (event: Event) ⇒ {
          observer.connect()
        }
        webSocket.onerror = (event: ErrorEvent) ⇒ {
          subscriber.onError(WebsocketObservable.Exception(event.message))
        }
        webSocket.onclose = (event: CloseEvent) ⇒ {
          subscriber.onComplete()
        }
        webSocket.onmessage = (event: MessageEvent) ⇒ {
          event.data match {
            case s: String ⇒
              subscriber.onError(
                WebsocketObservable.Exception(s"String message event is unsupported. Message: $s")
              )
            case buf: ArrayBuffer ⇒
              subscriber.onNext(Binary(arrayToByteVector(buf)))
            case b: Blob ⇒
              val fReader = new FileReader()
              fReader.onload = (event: UIEvent) ⇒ {
                val buf = fReader.result.asInstanceOf[ArrayBuffer]
                val bv = arrayToByteVector(buf)
                subscriber.onNext(Binary(bv))
              }
              fReader.readAsArrayBuffer(b)
          }

        }

        Cancelable(() ⇒ closeConnection(webSocket))
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

      def onError(ex: Throwable): Unit = {
        scheduler.reportFailure(ex)
        self
          .delaySubscription(3.seconds)
          .unsafeSubscribeFn(subscriber)
      }

      def onComplete(): Unit = {
        self
          .delaySubscription(3.seconds)
          .unsafeSubscribeFn(subscriber)
      }
    })
  }
}

object WebsocketObservable {

  case class Exception(msg: String) extends RuntimeException(msg)
}
