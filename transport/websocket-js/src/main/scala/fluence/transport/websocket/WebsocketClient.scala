package fluence.transport.websocket

import monix.execution.{Ack, Scheduler}
import monix.reactive._
import scodec.bits.ByteVector

import scala.concurrent.Future

object WebsocketClient {

  /**
   *
   * @param url Address to connect by websocket
   * @return An observer that will be an input into a websocket and observable - output
   */
  def apply(url: String)(implicit scheduler: Scheduler): (Observer[WebsocketFrame], Observable[WebsocketFrame]) = {

    val queueingSubject = new QueueingSubject[WebsocketFrame]

    val observable = new WebsocketObservable(url, queueingSubject)

    val hotObservable = observable.multicast(Pipe.publish[WebsocketFrame])
    hotObservable.connect()

    queueingSubject -> hotObservable
  }

  /**
   * Client that accepts only binary data.
   */
  def binaryClient(
    url: String
  )(implicit scheduler: Scheduler): (Observer[ByteVector], Observable[ByteVector]) = {

    val (wsObserver, wsObservable) = WebsocketClient(url)

    val binaryClient: Observer[ByteVector] = new Observer[ByteVector] {
      override def onNext(elem: ByteVector): Future[Ack] = wsObserver.onNext(Binary(elem))

      override def onError(ex: Throwable): Unit = wsObserver.onError(ex)

      override def onComplete(): Unit = wsObserver.onComplete()
    }

    val binaryObservable = wsObservable.collect {
      case Binary(data) ⇒ data
    }

    binaryClient -> binaryObservable
  }
}
