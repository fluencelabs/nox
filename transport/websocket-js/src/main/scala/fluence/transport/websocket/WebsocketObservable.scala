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
 * @param input Stream of events that we send by websocket.
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

  var lastFrame: Option[WebsocketFrame] = None

  // An observer that will send events from input observable
  private val inputObserver: Observer[(WebsocketT, WebsocketFrame)] = new Observer[(WebsocketT, WebsocketFrame)] {

    override def onNext(elem: (WebsocketT, WebsocketFrame)): Future[Ack] = {
      val websocket = elem._1
      val frame = elem._2
      val try_ = Try(sendFrame(websocket, frame))
      try_ match {
        case Success(_) ⇒
          lastFrame = None
        case Failure(ex) ⇒
          lastFrame = Option(frame)
      }
      Future { try_.get }.map(_ ⇒ Continue).recover {
        case e: Throwable ⇒
          logger.error("Unsupported exception", e)
          Stop
      }
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

        try {
          webSocket.setOnopen((event: Event) ⇒ {
            println("ON OPEN")
            attempts.set(0)
            cancelable = Some(input.map(fr ⇒ (webSocket, fr)).subscribe(inputObserver))
            lastFrame.foreach(fr ⇒ inputObserver.onNext((webSocket, fr)))
          })
        } catch {
          case e: Throwable ⇒ e.printStackTrace()
        }

        webSocket.setOnerror((event: ErrorEvent) ⇒ {
          logger.error(s"Error in websocket $url: ${event.message}")
          subscriber.onError(WebsocketObservable.WebsocketException(event.message))
        })

        webSocket.setOnclose((event: CloseEvent) ⇒ {
          cancelable.foreach(_.cancel())
          subscriber.onComplete()
        })

        webSocket.setOnmessage((event: MessageEvent) ⇒ {
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

      def tryReconnect(call: ⇒ Unit): Unit = {
        if (!closed.get && numberOfAttempts >= attempts.get) {
          attempts.increment()

          // Subscribing we reconnect to the websocket.
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
