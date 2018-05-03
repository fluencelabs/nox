package fluence.transport.websocket

import monix.execution.Ack.Continue
import monix.reactive.{Observable, Observer}
import org.scalatest.{Matchers, WordSpec}

import scala.concurrent.Future
import monix.execution.Scheduler.Implicits.global

import scala.collection.mutable

class QueueingSubjectSpec extends WordSpec with Matchers {
  "QueueingSubject" should {
    "buffer elements if no subscribers and emit buffered elements after subscribe" in {
      val subject = new QueueingSubject[String]

      val checker = mutable.ListBuffer.empty[String]

      val observer: Observer[String] = subject
      val observable: Observable[String] = subject

      observer.onNext("1")
      observer.onNext("2")

      val cancel = observable.subscribe(nextFn = { s ⇒
        checker += s
        Future(Continue)
      })

      observer.onNext("3")
      observer.onNext("4")

      cancel.cancel()

      observer.onNext("5")
      observer.onNext("6")

      val cancel2 = observable.subscribe(nextFn = { s ⇒
        checker += s
        Future(Continue)
      })

      observer.onNext("7")
      observer.onNext("8")

      checker should contain theSameElementsInOrderAs Range(1, 9).map(_.toString)
    }
  }
}
