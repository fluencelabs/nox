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

  val (observer, observable) = WebsocketClient.binaryClient[ByteVector, ByteVector](
    "ws://localhost:8080/http4s/wsecho",
    identity,
    identity
  )

  val pr11 = Promise[Unit]
  val pr12 = Promise[Unit]
  val pr21 = Promise[Unit]

  val obs1 = observable.subscribe(bv ⇒ {
    println("111 === " + bv.toArray.mkString(","))
    if (bv == ByteVector(1)) pr11.trySuccess(())
    if (bv == ByteVector(2)) pr12.trySuccess(())
    Future(Continue)
  })

  val obs2 = observable.subscribe(bv ⇒ {
    println("222 === " + bv.toArray.mkString(","))
    pr21.trySuccess(())
    Future(Continue)
  })

  observer.onNext(ByteVector(1))

  val pr31 = Promise[Unit]
  val pr41 = Promise[Unit]
  val pr51 = Promise[Unit]

  for {
    _ ← pr11.future
    _ ← pr21.future
    _ = obs2.cancel()
    _ = observer.onNext(ByteVector(2))
    _ ← pr12.future
    obs3 = observable.subscribe(bv ⇒ {
      println("333 === " + bv.toArray.mkString(","))
      if (bv == ByteVector(3)) pr31.trySuccess(())
      Future(Continue)
    })
    obs4 = observable.subscribe(bv ⇒ {
      println("444 === " + bv.toArray.mkString(","))
      if (bv == ByteVector(3)) pr41.trySuccess(())
      Future(Continue)
    })
    _ = observer.onNext(ByteVector(3))
    _ ← pr31.future
    _ ← pr41.future
    _ = obs1.cancel()
    _ = obs3.cancel()
    _ = obs4.cancel()
    obs5 = observable.subscribe(bv ⇒ {
      println("555 === " + bv.toArray.mkString(","))
      if (bv == ByteVector(4)) pr51.trySuccess(())
      Future(Continue)
    })
    _ = observer.onNext(ByteVector(4))
    _ ← pr51.future
  } yield {
    println("HEHEY")
  }

}
