package fluence.transport.websocket

import monix.execution.Ack.Continue
import monix.execution.{Ack, Scheduler}
import monix.reactive._
import org.scalajs.dom.{CloseEvent, Event, MessageEvent, WebSocket}
import scodec.bits.ByteVector
import monix.execution.Scheduler.Implicits.global

import scala.concurrent.{Await, Future, Promise}

object Main extends App {

  val overflow: OverflowStrategy.Synchronous[Nothing] = OverflowStrategy.Unbounded

  val (observer, observable) = WebsocketPipe.websocketPipe[ByteVector, ByteVector](
    "ws://localhost:8080/http4s/wsecho",
    implicitly[Scheduler],
    identity,
    identity
  )

  observable.subscribe(bv ⇒ {
    println("111 " + bv)
    Future(Continue)
  })
  observable.subscribe(bv ⇒ {
    println("222 " + bv)
    Future(Continue)
  })
  observable.subscribe(bv ⇒ {
    println("333 " + bv)
    Future(Continue)
  })
  observer.onNext(ByteVector(1, 2, 3, 4, 5))
  observer.onNext(ByteVector(1, 2, 3, 4, 5))
  observer.onNext(ByteVector(1, 2, 3, 4, 5))
  observer.onNext(ByteVector(1, 2, 3, 4, 5))
  observer.onNext(ByteVector(1, 2, 3, 4, 5))
  observable.subscribe(bv ⇒ {
    println("444 " + bv)
    Future(Continue)
  })
  observable.subscribe(bv ⇒ {
    println("555 " + bv)
    Future(Continue)
  })
  observable.subscribe(bv ⇒ {
    println("666  " + bv)
    Future(Continue)
  })

}
