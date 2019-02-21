package fluence.node.workers
import cats.effect.{ContextShift, IO}
import fluence.effects.kvstore.MVarKVStore
import org.scalatest.{Matchers, WordSpec}

import scala.concurrent.ExecutionContext.Implicits.global

class WorkersPortsSpec extends WordSpec with Matchers {
  implicit val ioShift: ContextShift[IO] = IO.contextShift(global)

  "workers ports" should {
    "allocate and deallocate" in {
      MVarKVStore
        .make[IO, Long, Short]()
        .use { store ⇒
          val minPort: Short = 100
          val maxPort: Short = 1000

          val workersPorts = WorkersPorts.make[IO](minPort, maxPort, store)

          for {

            _ ← workersPorts.use { ports ⇒
              for {

                init ← ports.getMapping
                _ = init shouldBe empty

                p1 ← ports.allocate(1l)
                _ = p1 shouldBe 100

                mapping1 ← ports.getMapping
                _ = mapping1 shouldBe Map(1l -> 100.toShort)

                pp1 ← ports.allocate(1l)
                _ = pp1 shouldBe 100

                pg1 ← ports.get(1l)
                _ = pg1 shouldBe Some(100)

                pgn ← ports.get(1000l)
                _ = pgn shouldBe empty

                _ ← ports.free(1l)

                p1f ← ports.get(1l)
                _ = p1f shouldBe empty

                freed ← ports.getMapping
                _ = freed shouldBe empty

                pp10 ← ports.allocate(10l)
                _ = pp10 shouldBe 100

                _ ← ports.allocate(20l)

              } yield ()

            }

            _ ← workersPorts.use(
              ports ⇒
                for {
                  init ← ports.getMapping
                  _ = init shouldBe Map(10l -> 100, 20l -> 101)
                } yield ()
            )

          } yield ()

        }
        .unsafeRunSync()
    }
  }
}
