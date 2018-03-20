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

package fluence.kad

import monix.eval.Task
import monix.execution.Scheduler.Implicits.global
import monix.execution.{CancelableFuture, Scheduler}
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.{Matchers, WordSpec}

import scala.util.{Failure, Success}

class MVarCacheSpec extends WordSpec with Matchers with ScalaFutures {

  //todo move it to utility module
  //todo add real concurrent tests
  private implicit class WaitTask[T](task: Task[T]) {

    def taskValue()(implicit s: Scheduler): T = {
      val future: CancelableFuture[T] = task.runAsync(s)
      future.onComplete {
        case Success(_) ⇒ ()
        case Failure(exception) ⇒ println(Console.RED + s"TASK ERROR: $exception")
      }(s)
      future.futureValue
    }
  }

  "some" should {
    "work" in {
      val cache = new MVarMapCache[Int, String]("")

      Task
        .sequence(
          Seq(
            cache.getOrAdd(1, "1"),
            cache.getOrAdd(1, "2"),
            cache.getOrAdd(1, "3")
          )
        )
        .taskValue()

      cache.get(1).get shouldBe "1"

      Task
        .sequence(
          Seq(
            cache.modify(1, s ⇒ s ++ "2"),
            cache.modify(1, s ⇒ s ++ "3"),
            cache.modify(1, s ⇒ s ++ "4")
          )
        )
        .taskValue()

      cache.get(1).get shouldBe "1234"

      Task
        .sequence(
          Seq(
            cache.update(1, "111"),
            cache.update(1, "222"),
            cache.update(1, "333")
          )
        )
        .taskValue()

      cache.get(1).get shouldBe "333"

      cache.modify(12, s ⇒ s ++ "2").taskValue() shouldBe false

      cache.getOrAddF(1, Task.pure("2")).taskValue() shouldBe "333"

      Task
        .sequence(
          Seq(
            cache.getOrAddF(4, Task.pure("444")),
            cache.getOrAddF(4, Task.pure("555")),
            cache.getOrAddF(4, Task.pure("666"))
          )
        )
        .taskValue()
      cache.getOrAddF(4, Task.pure("4")).taskValue() shouldBe "444"
    }
  }
}
