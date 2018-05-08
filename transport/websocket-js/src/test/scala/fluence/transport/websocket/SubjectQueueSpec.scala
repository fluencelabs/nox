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
import monix.reactive.{Observable, Observer}
import org.scalatest.{Matchers, WordSpec}

import scala.concurrent.Future
import monix.execution.Scheduler.Implicits.global

import scala.collection.mutable

class SubjectQueueSpec extends WordSpec with Matchers {
  "QueueingSubject" should {
    "buffer elements if no subscribers and emit buffered elements after subscribe" in {
      val subject = new SubjectQueue[String]

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
