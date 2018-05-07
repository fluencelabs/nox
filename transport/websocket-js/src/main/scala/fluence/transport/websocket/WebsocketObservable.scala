package fluence.transport.websocket

import monix.execution.Ack.{Continue, Stop}
import monix.execution.atomic.{AtomicBoolean, AtomicInt}
import monix.execution.{Ack, Cancelable, Scheduler}
import monix.reactive.observers.Subscriber
import monix.reactive.{Observable, Observer, OverflowStrategy}
import org.scalajs.dom._
import scodec.bits.ByteVector

import scala.concurrent.Future
import scala.concurrent.duration._
import scala.scalajs.js.JSConverters._
import scala.scalajs.js.typedarray.TypedArrayBufferOps._
import scala.scalajs.js.typedarray.{ArrayBuffer, Int8Array, TypedArrayBuffer}
import scala.util.{Failure, Success, Try}
import scala.util.control.{NoStackTrace, NonFatal}

/**
 * Output of websocket.
 * @param url Connecting with websocket.
 * @param input Stream of events that we send by websocket. Will subscribe on it only when websocket is available.
 * @param numberOfAttempts Number of attempts to reconnect if we have an error or the connection will break.
 * @param connectTimeout Timeout to reconnect.
 */
final class WebsocketObservable(
  url: String,
  builder: String ⇒ WebsocketT,
  input: Observable[WebsocketFrame],
  numberOfAttempts: Int = 13,
  connectTimeout: FiniteDuration = 3.seconds
)(
  implicit scheduler: Scheduler
) extends Observable[WebsocketFrame] with slogging.LazyLogging {
  self ⇒

  case class SendingElement(websocket: WebsocketT, frame: WebsocketFrame)

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

  /**
   * Close connection to websocket. This will cause the restart of observable.
   */
  private def closeConnection(webSocket: WebsocketT): Unit = {
    if (webSocket != null && webSocket.readyState <= 1)
      try webSocket.close()
      catch {
        case _: Throwable ⇒ ()
      }
  }

  private def sendFrame(ws: WebsocketT, fr: WebsocketFrame): Unit = {
    fr match {
      case Binary(data) ⇒
        val arr = new Int8Array(data.toArray.toJSArray)
        val buffer = TypedArrayBuffer.wrap(arr).arrayBuffer()
        ws.send(buffer)
      case Text(data) ⇒
        ws.send(data)
      case CloseFrame(cause) ⇒
        closed.set(true)
        closeConnection(ws)
    }
  }

  /**
   * Cached frame if we got exception on sending this frame.
   */
  var lastFrame: Option[WebsocketFrame] = None

  // An observer that will send events from input observable
  private val inputObserver: Observer[SendingElement] = new Observer[SendingElement] {

    override def onNext(elem: SendingElement): Future[Ack] = {
      val try_ = Try(sendFrame(elem.websocket, elem.frame))
      try_ match {
        case Success(_) ⇒
          lastFrame = None
        case Failure(ex) ⇒
          lastFrame = Option(elem.frame)
          logger.error(
            s"Unsupported exception on sending message through websocket to $url. Store frame and restart websocket.",
            ex
          )
          elem.websocket.close()
      }
      Future(Continue)
    }

    override def onError(ex: Throwable): Unit =
      logger.error(s"Unexpected error in observer in websocket $url", ex)

    override def onComplete(): Unit = ()
  }

  /** An `Observable` that upon subscription will open a
   *  web-socket connection.
   */
  private val channel: Observable[WebsocketFrame] =
    Observable.create[WebsocketFrame](overflow) { subscriber ⇒
      try {
        // Opening a WebSocket connection using Javascript's API
        logger.info(s"Connecting to $url")

        val webSocket = builder(url)

        // Input subscription
        var cancelable: Option[Cancelable] = None

        // Send exception to subscriber on error. This will restart observable and reconnect websocket.
        webSocket.setOnerror((event: ErrorEvent) ⇒ {
          logger.error(s"Error in websocket $url: ${event.message}")
          subscriber.onError(WebsocketObservable.WebsocketException(event.message))
        })

        // Send onComplete to subscriber. This will restart observable and reconnect websocket.
        webSocket.setOnclose((event: CloseEvent) ⇒ {
          logger.debug(s"OnClose event $event in websocket $url")
          cancelable.foreach(_.cancel())
          subscriber.onComplete()
        })

        webSocket.setOnmessage((event: MessageEvent) ⇒ {
          logger.debug(s"OnMessage event $event in websocket $url")
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
                logger.error(s"Unexpected error on reading binary frame from websocket $url}")
                subscriber.onError(WebsocketObservable.WebsocketException("Unexpected error on reading binary frame."))
              }

              fReader.readAsArrayBuffer(b)
          }

        })

        // Subscribe to incoming messages here. Send cached frame if it was error on sending message before.
        webSocket.setOnopen((event: Event) ⇒ {
          logger.debug(s"OnOpen event $event in websocket $url")
          attempts.set(0)
          cancelable = Some(input.map(fr ⇒ SendingElement(webSocket, fr)).subscribe(inputObserver))
          lastFrame.foreach(fr ⇒ inputObserver.onNext(SendingElement(webSocket, fr)))
        })

        Cancelable(() ⇒ {
          logger.info(s"Closing connection to $url")
        })
      } catch {
        case NonFatal(ex) ⇒
          subscriber.onError(ex)
          Cancelable.empty
      }
    }

  override def unsafeSubscribeFn(subscriber: Subscriber[WebsocketFrame]): Cancelable = {

    channel.unsafeSubscribeFn(new Observer[WebsocketFrame] {

      /**
       * We will reconnect until the observable is `closed` or until the `attempts` ends.
       */
      def tryReconnect(call: ⇒ Unit): Unit = {
        if (!closed.get && numberOfAttempts >= attempts.get) {
          attempts.increment()

          // By subscribing we reconnect to the websocket.
          val canc = self
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
