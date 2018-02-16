package fluence.node

import fluence.client.MVarMapCache
import monix.eval.Task
import monix.execution.{ CancelableFuture, Scheduler }
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.{ BeforeAndAfterAll, Matchers, WordSpec }

import monix.execution.Scheduler.Implicits.global

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

      val a = cache.getOrAdd(1, "1").taskValue()
      println(a)
      val b = cache.getOrAdd(1, "2").taskValue()
      println(b)
      val c = cache.modify(1, s ⇒ s ++ "2").taskValue()
      println(c)
      val e = cache.get(1)
      println(e)
      val f = cache.update(1, "123").taskValue()
      println(f)
      val g = cache.get(1)
      println(g)
      val d = cache.modify(12, s ⇒ s ++ "2").taskValue()
      println(d)

    }
  }
}
