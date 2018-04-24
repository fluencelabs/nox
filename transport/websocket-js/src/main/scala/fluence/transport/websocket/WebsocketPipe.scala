package fluence.transport.websocket

import monix.execution.Ack.Continue
import monix.execution.{Ack, Scheduler}
import monix.reactive.observables.ConnectableObservable
import monix.reactive.{Observable, Observer, Pipe}
import monix.reactive.observers.{CacheUntilConnectSubscriber, Subscriber}
import org.scalajs.dom.WebSocket
import scodec.bits.ByteVector

import scala.scalajs.js.JSConverters._
import scala.scalajs.js.typedarray.{ArrayBuffer, Int8Array, TypedArrayBuffer, Uint8Array}
import scala.scalajs.js.typedarray.TypedArrayBufferOps._
import scala.concurrent.Future
import scala.scalajs.js.typedarray.{Int8Array, TypedArrayBuffer}

object WebsocketPipe {

  def websocketPipe[I, O](
    url: String,
    sched: Scheduler,
    out: ByteVector ⇒ O,
    in: I ⇒ ByteVector
  ): (CacheUntilConnectSubscriber[I], ConnectableObservable[ByteVector]) = {

    val ws = new WebSocket(url)

    val fromClient: Subscriber[I] = new Subscriber[I] {
      override def onNext(elem: I): Future[Ack] = {
        val byteVector = in(elem)
        val arr = new Int8Array(byteVector.toArray.toJSArray)
        val buffer = TypedArrayBuffer.wrap(arr).arrayBuffer()
        ws.send(buffer)
        Future(Continue)
      }

      override def onError(ex: Throwable): Unit = ex.printStackTrace()

      override def onComplete(): Unit = println("on complete")

      override implicit def scheduler: Scheduler = sched
    }

    val observer: CacheUntilConnectSubscriber[I] = CacheUntilConnectSubscriber(fromClient)

    val observable = new BackPressuredWebSocketClient(ws, observer)

    val a = observable.multicast(Pipe.publish[ByteVector])(sched)
    a.connect()

    observer -> a
  }
}
