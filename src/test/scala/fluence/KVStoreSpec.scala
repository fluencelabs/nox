package fluence

import cats.Applicative
import org.scalatest.{ Matchers, WordSpec }
import org.scalatest.concurrent.{ Eventually, ScalaFutures }
import monix.cats.MonixToCatsConversions
import cats.implicits._
import fluence.dataset.{ KVStore, ServerModule }
import freestyle._
import freestyle.implicits._
import monix.eval.Task
import monix.execution.Scheduler.Implicits.global

class KVStoreSpec extends WordSpec with Matchers with ScalaFutures with Eventually
  with MonixToCatsConversions {

  import fluence.dataset.KVMemoryHandler._

  "KVStore" should {

    def get[F[_]: KVStore](k: String): FreeS[F, Option[String]] = {
      val KV = implicitly[KVStore[F]]

      KV.get(k.getBytes("UTF-8")).map(_.map(new String(_, "UTF-8")))
    }

    def put[F[_]: KVStore](k: String, v: String): FreeS[F, Boolean] = {
      val KV = implicitly[KVStore[F]]

      get[F](k).flatMap {
        case Some(`v`) ⇒ FreeS.pure(false)
        case _         ⇒ KV.put(k.getBytes("UTF-8"), v.getBytes("UTF-8"))
      }

    }

    implicit class taskVal[T](program: FreeS[ServerModule.Op, T]) {
      def runValue: T = program.interpret[Task].runAsync.futureValue
    }

    "work" in {

      get[ServerModule.Op]("some key").taskVal should be('empty)

      put[ServerModule.Op]("some key", "some value").interpret[Task].runAsync.futureValue should be(true)

      eventually {
        get[ServerModule.Op]("some key").interpret[Task].runAsync.futureValue should be(Some("some value"))
      }

      put[ServerModule.Op]("some key", "some value").interpret[Task].runAsync.futureValue should be(false)

    }

  }

}
