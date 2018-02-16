package fluence.kad

import monix.eval.Task
import monix.execution.Scheduler.Implicits.global
import monix.execution.{ CancelableFuture, Scheduler }
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.{ Matchers, WordSpec }

import scala.util.{ Failure, Success }

class MVarCacheTest extends WordSpec with Matchers with ScalaFutures {

  //todo move it to utility module
  //todo add real concurrent tests
  private implicit class WaitTask[T](task: Task[T]) {
    def taskValue()(implicit s: Scheduler): T = {
      val future: CancelableFuture[T] = task.runAsync(s)
      future.onComplete {
        case Success(_)         ⇒ ()
        case Failure(exception) ⇒ println(Console.RED + s"TASK ERROR: $exception")
      }(s)
      future.futureValue
    }
  }

  "some" should {
    "work" in {
      val cache = new MVarMapCache[Int, String]("")

      Task.sequence(Seq(
        cache.getOrAdd(1, "1"),
        cache.getOrAdd(1, "2"),
        cache.getOrAdd(1, "3")
      )).taskValue()

      cache.get(1).get shouldBe "1"

      Task.sequence(Seq(
        cache.modify(1, s ⇒ s ++ "2"),
        cache.modify(1, s ⇒ s ++ "3"),
        cache.modify(1, s ⇒ s ++ "4")
      )).taskValue()

      cache.get(1).get shouldBe "1234"

      Task.sequence(Seq(
        cache.update(1, "111"),
        cache.update(1, "222"),
        cache.update(1, "333")
      )).taskValue()

      cache.get(1).get shouldBe "333"

      cache.modify(12, s ⇒ s ++ "2").taskValue() shouldBe false

      cache.getOrAddF(1, Task.pure("2")).taskValue() shouldBe "333"

      Task.sequence(Seq(
        cache.getOrAddF(4, Task.pure("444")),
        cache.getOrAddF(4, Task.pure("555")),
        cache.getOrAddF(4, Task.pure("666"))
      )).taskValue()
      cache.getOrAddF(4, Task.pure("4")).taskValue() shouldBe "444"
    }
  }
}
