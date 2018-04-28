package fluence.transport.websocket

import monix.execution.Ack.{Continue, Stop}
import monix.execution.atomic.{AtomicBoolean, AtomicInt}
import monix.execution.{Ack, Cancelable, Scheduler}
import monix.reactive.observers.{CacheUntilConnectSubscriber, Subscriber}
import monix.reactive.{Observable, Observer, OverflowStrategy}
import org.scalajs.dom._
import scodec.bits.ByteVector

import scala.concurrent.Future
import scala.concurrent.duration._
import scala.scalajs.js.JSConverters._
import scala.scalajs.js.typedarray.TypedArrayBufferOps._
import scala.scalajs.js.typedarray.{ArrayBuffer, Int8Array, TypedArrayBuffer}
import scala.util.control.{NoStackTrace, NonFatal}

/**
 * Ouput of websocket.
 */
final class WebsocketObservable(url: String, input: Observable[WebsocketFrame], numberOfAttempts: Int = 13)(
  implicit scheduler: Scheduler
) extends Observable[WebsocketFrame] with slogging.LazyLogging {
  self ⇒

  private val closed = AtomicBoolean(false)
  private val attempts = AtomicInt(0)

  //TODO control overflow strategy
  private val overflow: OverflowStrategy.Synchronous[Nothing] = OverflowStrategy.Unbounded

  private def arrayToByteVector(buf: ArrayBuffer): ByteVector = {
    val typed = TypedArrayBuffer.wrap(buf)
    ByteVector(typed)
  }

  private val channel: Observable[WebsocketFrame] =
    Observable.create[WebsocketFrame](overflow) { subscriber ⇒
      def closeConnection(webSocket: WebSocket): Unit = {
        if (webSocket != null && webSocket.readyState <= 1)
          try webSocket.close()
          catch {
            case _: Throwable ⇒ ()
          }
      }

      try {
        val webSocket = new WebSocket(url)

        logger.info(s"Connecting to ${webSocket.url}")

        val obs = new Observer[WebsocketFrame] {
          override def onNext(elem: WebsocketFrame): Future[Ack] = {
            Future {
              elem match {
                case Binary(data) ⇒
                  val arr = new Int8Array(data.toArray.toJSArray)
                  val buffer = TypedArrayBuffer.wrap(arr).arrayBuffer()
                  webSocket.send(buffer)
                case Text(data) ⇒
                  webSocket.send(data)
                case CloseFrame(cause) ⇒
                  closed.set(true)
                  closeConnection(webSocket)
              }
            }.map(_ ⇒ Continue).recover {
              case e: Throwable ⇒
                logger.error("Unsupported exception", e)
                Stop
            }
          }

          override def onError(ex: Throwable): Unit =
            logger.error(s"Unexpected error in observer in websocket ${webSocket.url}", ex)

          override def onComplete(): Unit = ()
        }

        val cacheUntilConnect = CacheUntilConnectSubscriber(Subscriber(obs, scheduler))

        val cancelable = input.subscribe(cacheUntilConnect)

        webSocket.onopen = (event: Event) ⇒ {
          attempts.set(0)
          cacheUntilConnect.connect()
        }
        webSocket.onerror = (event: ErrorEvent) ⇒ {
          logger.error(s"Error in websocket ${webSocket.url}: ${event.message}")
          subscriber.onError(WebsocketObservable.WebsocketException(event.message))
        }
        webSocket.onclose = (event: CloseEvent) ⇒ {
          cancelable.cancel()
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

        })
      } catch {
        case NonFatal(ex) ⇒
          subscriber.onError(ex)
          Cancelable.empty
      }
    }

  override def unsafeSubscribeFn(subscriber: Subscriber[WebsocketFrame]): Cancelable = {

    channel.unsafeSubscribeFn(new Observer[WebsocketFrame] {

      def tryReconnect(call: ⇒ Unit): Unit = {
        if (!closed.get && numberOfAttempts >= attempts.get) {
          attempts.increment()
          self
            .delaySubscription(3.seconds)
            .unsafeSubscribeFn(subscriber)
        }
      }

      def onNext(elem: WebsocketFrame): Future[Ack] = subscriber.onNext(elem)

      def onError(ex: Throwable): Unit = tryReconnect(subscriber.onError(ex))

      def onComplete(): Unit = tryReconnect(subscriber.onComplete())
    })
  }
}

object WebsocketObservable {

  case class WebsocketException(errorMessage: String, causedBy: Option[Throwable] = None)
      extends Throwable(errorMessage) with NoStackTrace {

    override def getCause: Throwable = causedBy getOrElse super.getCause
  }
}
