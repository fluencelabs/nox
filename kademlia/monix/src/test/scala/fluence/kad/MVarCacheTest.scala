package fluence.kad

import monix.eval.Task
import monix.execution.Scheduler.Implicits.global
import monix.execution.{ CancelableFuture, Scheduler }
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.{ Matchers, WordSpec }

import scala.util.{ Failure, Success }

class MVarCacheTest extends WordSpec with Matchers with ScalaFutures {

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

      cache.getOrAdd(1, "1").taskValue() shouldBe "1"

      cache.getOrAdd(1, "2").taskValue() shouldBe "1"

      cache.modify(1, s ⇒ s ++ "2").taskValue() shouldBe true
      cache.get(1).get shouldBe "12"

      cache.update(1, "123").taskValue()
      cache.get(1).get shouldBe "123"

      cache.modify(12, s ⇒ s ++ "2").taskValue() shouldBe false

      cache.getOrAddF(1, Task.pure("2")).taskValue() shouldBe "123"

      cache.getOrAddF(4, Task.pure("4")).taskValue() shouldBe "4"

    }
  }
}
