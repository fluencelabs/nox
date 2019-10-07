/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package fluence.node.workers
import cats.effect.{ContextShift, IO, Timer}
import fluence.effects.kvstore.MVarKVStore
import fluence.log.{Log, LogFactory}
import org.scalatest.{Matchers, WordSpec}

import scala.concurrent.ExecutionContext.Implicits.global

class WorkersPortsSpec extends WordSpec with Matchers {
  implicit val ioShift: ContextShift[IO] = IO.contextShift(global)

  implicit val timer: Timer[IO] = IO.timer(global)
  implicit val log: Log[IO] = LogFactory.forPrintln[IO]().init("WorkersPortsSpec").unsafeRunSync()

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

                p1 ← ports.allocate(1L).value.map(_.right.get)
                _ = p1 shouldBe 100

                mapping1 ← ports.getMapping
                _ = mapping1 shouldBe Map(1L -> 100.toShort)

                pp1 ← ports.allocate(1L).value.map(_.right.get)
                _ = pp1 shouldBe 100

                pg1 ← ports.get(1L)
                _ = pg1 shouldBe Some(100)

                pgn ← ports.get(1000L)
                _ = pgn shouldBe empty

                _ ← ports.free(1L).value.map(_.right.get)

                p1f ← ports.get(1L)
                _ = p1f shouldBe empty

                freed ← ports.getMapping
                _ = freed shouldBe empty

                pp10 ← ports.allocate(10L).value.map(_.right.get)
                _ = pp10 shouldBe 100

                _ ← ports.allocate(20L).value.map(_.right.get)

              } yield ()

            }

            _ ← workersPorts.use(
              ports ⇒
                for {
                  init ← ports.getMapping
                  _ = init shouldBe Map(10L -> 100, 20L -> 101)
                } yield ()
            )

          } yield ()

        }
        .unsafeRunSync()
    }
  }
}
