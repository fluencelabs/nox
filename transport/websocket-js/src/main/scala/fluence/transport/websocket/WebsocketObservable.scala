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
 * Output of websocket.
 * @param url Connecting with websocket.
 * @param input Stream of events that we send by websocket.
 * @param numberOfAttempts Number of attempts to reconnect if we have an error or the connection will break.
 * @param connectTimeout Timeout to reconnect.
 */
final class WebsocketObservable(
  url: String,
  input: Observable[WebsocketFrame],
  numberOfAttempts: Int = 13,
  connectTimeout: FiniteDuration = 3.seconds
)(
  implicit scheduler: Scheduler
) extends Observable[WebsocketFrame] with slogging.LazyLogging {
  self ⇒

  /**
   * Flag, that can be changed to `true` by `CloseFrame`.
   * If it is `true`, websocket can't reconnect and receive messages.
   * Observable is only for deleting by GC.
   */
  private val closed = AtomicBoolean(false)

  /**
   * Number of reconnecting attempts.
   */
  private val attempts = AtomicInt(0)

  //TODO control overflow strategy
  private val overflow: OverflowStrategy.Synchronous[Nothing] = OverflowStrategy.Unbounded

  private def arrayToByteVector(buf: ArrayBuffer): ByteVector = {
    val typed = TypedArrayBuffer.wrap(buf)
    ByteVector(typed)
  }

  /** An `Observable` that upon subscription will open a
   *  web-socket connection.
   */
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
        // Opening a WebSocket connection using Javascript's API
        logger.info(s"Connecting to $url")
        val webSocket = new WebSocket(url)

        // An observer that will send events from input observable
        val inputObserver = new Observer[WebsocketFrame] {
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

        // We cannot send messages until websocket does onopen event.
        // So, we will buffer messages until we call method `connect`.
        val cacheUntilConnect = CacheUntilConnectSubscriber(Subscriber(inputObserver, scheduler))

        // Subscribe on input. Messages began to be accepted by observer.
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
              // We get binary data in websockets usually in blobs.
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
          // Subscribing we reconnect to the websocket.
          self
            .delaySubscription(connectTimeout)
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
