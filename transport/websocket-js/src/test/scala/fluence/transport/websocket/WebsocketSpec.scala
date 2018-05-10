/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

package fluence.transport.websocket

import monix.execution.Ack.Continue
import monix.reactive.OverflowStrategy
import org.scalatest.{Assertion, AsyncFlatSpec, Matchers}
import scodec.bits.ByteVector

import scala.concurrent.{Future, Promise}
import scala.util.Try

class WebsocketSpec extends AsyncFlatSpec with Matchers {

  implicit override def executionContext = monix.execution.Scheduler.Implicits.global

  it should "work with multiple subscribers" in {
    val overflow: OverflowStrategy.Synchronous[Nothing] = OverflowStrategy.Unbounded

    val WebsocketClient(observer, observable, _) = WebsocketClient.binaryClient(
      "ws://localhost:8080/",
      s ⇒ WebsocketEcho(s)
    )

    val pr11 = Promise[Unit]
    val pr12 = Promise[Unit]
    val pr21 = Promise[Unit]

    val obs1 = observable.subscribe(bv ⇒ {
      if (bv sameElements Array[Byte](1)) pr11.trySuccess(())
      if (bv sameElements Array[Byte](2)) pr12.trySuccess(())
      Future(Continue)
    })

    val obs2 = observable.subscribe(bv ⇒ {
      pr21.trySuccess(())
      Future(Continue)
    })

    observer.onNext(Array[Byte](1))

    val pr31 = Promise[Unit]
    val pr41 = Promise[Unit]
    val pr51 = Promise[Unit]

    val resultPromise = Promise[Assertion]

    (for {
      _ ← pr11.future
      _ ← pr21.future
      _ = obs2.cancel()
      _ = observer.onNext(Array[Byte](2))
      _ ← pr12.future
      obs3 = observable.subscribe(bv ⇒ {
        if (bv sameElements Array[Byte](3)) pr31.trySuccess(())
        Future(Continue)
      })
      obs4 = observable.subscribe(bv ⇒ {
        if (bv sameElements Array[Byte](3)) pr41.trySuccess(())
        Future(Continue)
      })
      _ = observer.onNext(Array[Byte](3))
      _ ← pr31.future
      _ ← pr41.future
      _ = obs1.cancel()
      _ = obs3.cancel()
      _ = obs4.cancel()
      obs5 = observable.subscribe(bv ⇒ {
        if (bv sameElements Array[Byte](4)) pr51.trySuccess(())
        Future(Continue)
      })
      _ = observer.onNext(Array[Byte](4))
      _ ← pr51.future
    } yield {
      val obs6 = observable.subscribe(bv ⇒ {
        Future(Continue)
      })
      assert(true)
    }).recover {
      case e: Throwable ⇒
        e.printStackTrace()
        assert(false)
    }
  }

  it should "not lose message after exception on send" in {
    val overflow: OverflowStrategy.Synchronous[Nothing] = OverflowStrategy.Unbounded

    val pr11 = Promise[Unit]
    val pr21 = Promise[Unit]

    val WebsocketClient(observer, observable, _) = WebsocketClient.binaryClient(
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

    observer.onNext(WebsocketEcho.errorOnceOnSendMessage)

    (for {
      _ ← pr11.future
      _ ← pr21.future
    } yield {
      assert(true)
    }).recover {
      case e: Throwable ⇒
        e.printStackTrace()
        assert(false)
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

    val WebsocketClient(observer, observable, _) = WebsocketClient.binaryClient(
      "ws://localhost:8080/",
      s ⇒ WebsocketEcho(s)
    )

    val obs1 = observable.subscribe(bv ⇒ {
      if (bv sameElements Array[Byte](1)) pr11.trySuccess(())
      if (bv sameElements Array[Byte](2)) pr12.trySuccess(())
      if (bv sameElements Array[Byte](3)) pr13.trySuccess(())
      if (bv sameElements Array[Byte](4)) pr14.trySuccess(())
      Future(Continue)
    })

    val obs2 = observable.subscribe(bv ⇒ {
      if (bv sameElements Array[Byte](1)) pr21.trySuccess(())
      if (bv sameElements Array[Byte](2)) pr22.trySuccess(())
      if (bv sameElements Array[Byte](3)) pr23.trySuccess(())
      if (bv sameElements Array[Byte](4)) pr24.trySuccess(())
      Future(Continue)
    })

    observer.onNext(Array[Byte](1))
    observer.onNext(Array[Byte](2))
    observer.onNext(WebsocketEcho.closeWebsocketMessage)
    observer.onNext(Array[Byte](3))
    observer.onNext(Array[Byte](4))

    (for {
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
    }).recover {
      case e: Throwable ⇒
        e.printStackTrace()
        assert(false)
    }
  }

  "error status" should "be sent to status observable" in {
    val overflow: OverflowStrategy.Synchronous[Nothing] = OverflowStrategy.Unbounded

    val prOnOpen = Promise[Unit]
    val prOnError = Promise[Unit]

    val WebsocketClient(observer, observable, statusOutput) = WebsocketClient.binaryClient(
      "ws://localhost:8080/",
      s ⇒ WebsocketEcho(s)
    )

    val obs1 = statusOutput.subscribe(status ⇒ {
      status match {
        case WebsocketOnOpen ⇒ prOnOpen.success(())
        case WebsocketOnError(_) ⇒ prOnError.success(())
        case _ ⇒
      }
      Future(Continue)
    })

    (for {
      _ ← prOnOpen.future
      _ = observer.onNext(WebsocketEcho.errorWebsocketMessage)
      _ ← prOnError.future
    } yield {
      obs1.cancel()
      assert(true)
    }).recover {
      case e: Throwable ⇒
        e.printStackTrace()
        assert(false)
    }
  }

  "close status" should "be sent to status observable" in {
    val overflow: OverflowStrategy.Synchronous[Nothing] = OverflowStrategy.Unbounded

    val prOnOpen = Promise[Unit]
    val prOnOpen2 = Promise[Unit]
    val prOnClose = Promise[Unit]

    val WebsocketClient(observer, observable, statusOutput) = WebsocketClient.binaryClient(
      "ws://localhost:8080/",
      s ⇒ WebsocketEcho(s)
    )

    val obs1 = statusOutput.subscribe(status ⇒ {
      status match {
        case WebsocketOnOpen if prOnOpen.isCompleted ⇒ prOnOpen2.success(())
        case WebsocketOnOpen ⇒ prOnOpen.success(())
        case WebsocketOnClose(_, _) if !prOnClose.isCompleted ⇒ prOnClose.success(())
        case _ ⇒
      }
      Future(Continue)
    })

    (for {
      _ ← prOnOpen.future
      _ = observer.onNext(WebsocketEcho.closeWebsocketMessage)
      _ ← prOnClose.future
      _ ← prOnOpen2.future
    } yield {
      obs1.cancel()
      assert(true)
    }).recover {
      case e: Throwable ⇒
        e.printStackTrace()
        assert(false)
    }
  }

  "websocket" should "be restarted on error on send" in {
    val overflow: OverflowStrategy.Synchronous[Nothing] = OverflowStrategy.Unbounded

    val prOnOpen = Promise[Unit]
    val prOnOpen2 = Promise[Unit]
    val prOnClose = Promise[Unit]

    val WebsocketClient(observer, observable, statusOutput) = WebsocketClient.binaryClient(
      "ws://localhost:8080/",
      s ⇒ WebsocketEcho(s)
    )

    val obs1 = statusOutput.subscribe(status ⇒ {
      status match {
        case WebsocketOnOpen if prOnOpen.isCompleted ⇒ prOnOpen2.success(())
        case WebsocketOnOpen ⇒ prOnOpen.success(())
        case WebsocketOnClose(_, _) if !prOnClose.isCompleted ⇒ prOnClose.success(())
        case _ ⇒
      }
      Future(Continue)
    })

    (for {
      _ ← prOnOpen.future
      _ = observer.onNext(WebsocketEcho.errorOnceOnSendMessage)
      _ ← prOnClose.future
      _ ← prOnOpen2.future
    } yield {
      obs1.cancel()
      assert(true)
    }).recover {
      case e: Throwable ⇒
        e.printStackTrace()
        assert(false)
    }
  }
}
