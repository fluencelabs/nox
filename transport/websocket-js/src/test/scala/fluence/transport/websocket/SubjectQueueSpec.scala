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
import org.scalatest.{AsyncFlatSpec, Matchers}

import scala.concurrent.Future

import scala.collection.mutable

class SubjectQueueSpec extends AsyncFlatSpec with Matchers {

  implicit override def executionContext = monix.execution.Scheduler.Implicits.global

  "SubjectQueue" should "buffer elements if no subscribers and emit buffered elements after subscribe" in {
    val subject = new SubjectQueue[String]

    val checker = mutable.ListBuffer.empty[String]

    val observer: Observer[String] = subject
    val observable: Observable[String] = subject

    for {
      _ ← observer.onNext("1")
      _ ← observer.onNext("2")
      cancel = observable.subscribe(nextFn = { s ⇒
        checker += s
        Future(Continue)
      })
      _ ← observer.onNext("3")
      _ ← observer.onNext("4")
      _ = cancel.cancel()
      _ ← observer.onNext("5")
      _ ← observer.onNext("6")
      cancel2 = observable.subscribe(nextFn = { s ⇒
        checker += s
        Future(Continue)
      })
      _ ← observer.onNext("7")
      _ ← observer.onNext("8")
    } yield {
      checker should contain theSameElementsInOrderAs Range(1, 9).map(_.toString)
    }
  }
}
