package fluence.transport.websocket

import monix.execution.{Ack, Scheduler}
import monix.reactive.{Observable, Observer, Pipe}
import monix.reactive.observers.{CacheUntilConnectSubscriber, Subscriber}
import org.scalajs.dom.WebSocket
import scodec.bits.ByteVector

import scala.concurrent.Future

object WebsocketClient {

  def apply(url: String)(implicit scheduler: Scheduler): (Observer[WebsocketFrame], Observable[WebsocketFrame]) = {

    val ws = new WebSocket(url)

    val observer = new WebsocketObserver(ws)

    val cacheUntilConnectSubscriber = CacheUntilConnectSubscriber(Subscriber(observer, scheduler))

    val observable = new WebsocketObservable(ws, cacheUntilConnectSubscriber)

    val coldObservable = observable.multicast(Pipe.publish[WebsocketFrame])
    coldObservable.connect()

    cacheUntilConnectSubscriber -> coldObservable
  }

  def binaryClient[I, O](
    url: String,
    out: ByteVector ⇒ O,
    in: I ⇒ ByteVector
  )(implicit scheduler: Scheduler): (Observer[I], Observable[ByteVector]) = {

    val (wsObserver, wsObservable) = WebsocketClient(url)

    val binaryClient: Observer[I] = new Observer[I] {
      override def onNext(elem: I): Future[Ack] = {
        val byteVector = in(elem)
        wsObserver.onNext(Binary(byteVector))
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
