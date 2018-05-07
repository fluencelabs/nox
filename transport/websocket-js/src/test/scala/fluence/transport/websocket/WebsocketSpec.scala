package fluence.transport.websocket

import monix.execution.Ack.Continue
import monix.reactive.OverflowStrategy
import org.scalatest.{Assertion, AsyncFlatSpec, Matchers}
import scodec.bits.ByteVector
import slogging.{LogLevel, LoggerConfig, PrintLoggerFactory}

import scala.concurrent.{Future, Promise}
import scala.util.Try

class WebsocketSpec extends AsyncFlatSpec with Matchers {

  implicit override def executionContext = monix.execution.Scheduler.Implicits.global

//  LoggerConfig.factory = PrintLoggerFactory()
//  LoggerConfig.level = LogLevel.DEBUG

  it should "work with multiple subscribers" in {
    val overflow: OverflowStrategy.Synchronous[Nothing] = OverflowStrategy.Unbounded

    val (observer, observable) = WebsocketClient.binaryClient(
      "ws://localhost:8080/",
      s ⇒ WebsocketEcho(s)
    )

    val pr11 = Promise[Unit]
    val pr12 = Promise[Unit]
    val pr21 = Promise[Unit]

    val obs1 = observable.subscribe(bv ⇒ {
      if (bv == ByteVector(1)) pr11.trySuccess(())
      if (bv == ByteVector(2)) pr12.trySuccess(())
      Future(Continue)
    })

    val obs2 = observable.subscribe(bv ⇒ {
      pr21.trySuccess(())
      Future(Continue)
    })

    Try(observer.onNext(ByteVector(1)))

    val pr31 = Promise[Unit]
    val pr41 = Promise[Unit]
    val pr51 = Promise[Unit]

    val resultPromise = Promise[Assertion]

    for {
      _ ← pr11.future
      _ ← pr21.future
      _ = obs2.cancel()
      _ = observer.onNext(ByteVector(2))
      _ ← pr12.future
      obs3 = observable.subscribe(bv ⇒ {
        if (bv == ByteVector(3)) pr31.trySuccess(())
        Future(Continue)
      })
      obs4 = observable.subscribe(bv ⇒ {
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
        if (bv == ByteVector(4)) pr51.trySuccess(())
        Future(Continue)
      })
      _ = observer.onNext(ByteVector(4))
      _ ← pr51.future
    } yield {
      val obs6 = observable.subscribe(bv ⇒ {
        Future(Continue)
      })
      resultPromise.trySuccess(assert(true))
    }

    resultPromise.future
  }

  it should "not lose message after exception on send" in {
    val overflow: OverflowStrategy.Synchronous[Nothing] = OverflowStrategy.Unbounded

    val pr11 = Promise[Unit]
    val pr21 = Promise[Unit]

    val (observer, observable) = WebsocketClient.binaryClient(
      "ws://localhost:8080/",
      s ⇒ WebsocketEcho(s)
    )

    val obs1 = observable.subscribe(bv ⇒ {
      pr11.trySuccess(())
      println("OBS1 === " + bv)
      Future(Continue)
    })

    val obs2 = observable.subscribe(bv ⇒ {
      println("OBS2 === " + bv)
      pr21.trySuccess(())
      Future(Continue)
    })

    Try(observer.onNext(ByteVector.apply(WebsocketEcho.errorOnceArray)))

    for {
      _ ← pr11.future
      _ ← pr21.future
    } yield {
      assert(true)
    }
  }
}
