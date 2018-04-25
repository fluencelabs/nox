package fluence.transport.websocket

import monix.execution.{Ack, Scheduler}
import monix.reactive.{Observable, Observer, Pipe}
import monix.reactive.observers.{CacheUntilConnectSubscriber, Subscriber}
import org.scalajs.dom.WebSocket
import scodec.bits.ByteVector

import scala.concurrent.Future

object WebsocketClient {

  /**
   *
   * @param url Address to connect by websocket
   * @return An observer that will be an input into a websocket and observable - output
   */
  def apply(url: String)(implicit scheduler: Scheduler): (Observer[WebsocketFrame], Observable[WebsocketFrame]) = {

    //TODO use https://github.com/joewalnes/reconnecting-websocket for stable websocket reconnection
    val ws = new WebSocket(url)

    val observer = new WebsocketObserver(ws)

    val cacheUntilConnectSubscriber = CacheUntilConnectSubscriber(Subscriber(observer, scheduler))

    val observable = new WebsocketObservable(ws, cacheUntilConnectSubscriber)

    val coldObservable = observable.multicast(Pipe.publish[WebsocketFrame])
    coldObservable.connect()

    cacheUntilConnectSubscriber -> coldObservable
  }

  /**
   * Client that accepts only binary data.
   */
  def binaryClient(
    url: String
  )(implicit scheduler: Scheduler): (Observer[ByteVector], Observable[ByteVector]) = {

    val (wsObserver, wsObservable) = WebsocketClient(url)

    val binaryClient: Observer[ByteVector] = new Observer[ByteVector] {
      override def onNext(elem: ByteVector): Future[Ack] = {
        wsObserver.onNext(Binary(elem))
      }

      override def onError(ex: Throwable): Unit = wsObserver.onError(ex)

      override def onComplete(): Unit = wsObserver.onComplete()
    }

    val binaryObservable = wsObservable.collect {
      case Binary(data) ⇒ data
    }

    binaryClient -> binaryObservable
  }
}
