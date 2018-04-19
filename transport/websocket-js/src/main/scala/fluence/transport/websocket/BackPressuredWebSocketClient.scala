package fluence.transport.websocket

import monix.execution.{Ack, Cancelable}
import monix.execution.rstreams.Subscription
import monix.reactive.{Observable, Observer}
import monix.reactive.observers.Subscriber
import org.scalajs.dom._

import scala.concurrent.Future
import scala.concurrent.duration._
import scala.util.control.NonFatal

final class BackPressuredWebSocketClient private (url: String) extends Observable[String] { self ⇒

  private val channel: Observable[String] =
    Observable.unsafeCreate { subscriber ⇒
      def closeConnection(webSocket: WebSocket): Unit = {
        println(s"Closing connection to $url")
        if (webSocket != null && webSocket.readyState <= 1)
          try webSocket.close()
          catch {
            case _: Throwable ⇒ ()
          }
      }

      val downstream = Subscriber.toReactiveSubscriber(subscriber)

      try {
        println(s"Connecting to $url")
        val webSocket = new WebSocket(url)

        webSocket.onopen = (event: Event) ⇒ {
          downstream.onSubscribe(new Subscription {
            def cancel(): Unit =
              closeConnection(webSocket)
            def request(n: Long): Unit = {
              webSocket.send(n.toString)
            }
          })
        }
        webSocket.onerror = (event: ErrorEvent) ⇒ {
          downstream.onError(BackPressuredWebSocketClient.Exception(event.message))
        }
        webSocket.onclose = (event: CloseEvent) ⇒ {
          downstream.onComplete()
        }
        webSocket.onmessage = (event: MessageEvent) ⇒ {
          downstream.onNext(event.data.asInstanceOf[String])
        }

        Cancelable(() ⇒ closeConnection(webSocket))
      } catch {
        case NonFatal(ex) ⇒
          downstream.onError(ex)
          Cancelable.empty
      }
    }

  override def unsafeSubscribeFn(subscriber: Subscriber[String]): Cancelable = {
    import subscriber.scheduler

    channel.unsafeSubscribeFn(new Observer[String] {
      def onNext(elem: String): Future[Ack] =
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

object BackPressuredWebSocketClient {

  def apply(url: String): BackPressuredWebSocketClient = {
    new BackPressuredWebSocketClient(url)
  }

  case class Exception(msg: String) extends RuntimeException(msg)
}
