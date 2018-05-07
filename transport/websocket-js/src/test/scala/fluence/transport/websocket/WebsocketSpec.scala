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
      Future(Continue)
    })

    val obs2 = observable.subscribe(bv ⇒ {
      pr21.trySuccess(())
      Future(Continue)
    })

    Try(observer.onNext(ByteVector.apply(WebsocketEcho.errorOnceMessage)))

    for {
      _ ← pr11.future
      _ ← pr21.future
    } yield {
      assert(true)
    }
  }

  it should "work if server restarted" in {
    val overflow: OverflowStrategy.Synchronous[Nothing] = OverflowStrategy.Unbounded

    val pr11 = Promise[Unit]
    val pr12 = Promise[Unit]
    val pr13 = Promise[Unit]
    val pr14 = Promise[Unit]
    val pr21 = Promise[Unit]
    val pr22 = Promise[Unit]
    val pr23 = Promise[Unit]
    val pr24 = Promise[Unit]

    val (observer, observable) = WebsocketClient.binaryClient(
      "ws://localhost:8080/",
      s ⇒ WebsocketEcho(s)
    )

    val obs1 = observable.subscribe(bv ⇒ {
      if (bv == ByteVector(1)) pr11.trySuccess(())
      if (bv == ByteVector(2)) pr12.trySuccess(())
      if (bv == ByteVector(3)) pr13.trySuccess(())
      if (bv == ByteVector(4)) pr14.trySuccess(())
      Future(Continue)
    })

    val obs2 = observable.subscribe(bv ⇒ {
      if (bv == ByteVector(1)) pr21.trySuccess(())
      if (bv == ByteVector(2)) pr22.trySuccess(())
      if (bv == ByteVector(3)) pr23.trySuccess(())
      if (bv == ByteVector(4)) pr24.trySuccess(())
      Future(Continue)
    })

    observer.onNext(ByteVector(1))
    observer.onNext(ByteVector(2))
    observer.onNext(ByteVector.apply(WebsocketEcho.closeWebsocketMessage))
    observer.onNext(ByteVector(3))
    observer.onNext(ByteVector(4))

    for {
      _ ← pr11.future
      _ ← pr12.future
      _ ← pr13.future
      _ ← pr14.future
      _ ← pr21.future
      _ ← pr22.future
      _ ← pr23.future
      _ ← pr24.future
    } yield {
      assert(true)
    }
  }
}
