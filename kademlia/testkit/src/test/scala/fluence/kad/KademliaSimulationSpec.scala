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

package fluence.kad

import java.time.Instant

import cats.syntax.show._
import cats.Show
import cats.data.EitherT
import cats.effect.{ContextShift, IO, Timer}
import cats.effect.concurrent.{MVar, Ref}
import fluence.kad.protocol.{Key, Node}
import fluence.kad.testkit.TestKademlia
import fluence.log.{ChainLog, Context, Log, PrintlnLog}
import org.scalatest.{Matchers, WordSpec}
import scodec.bits.ByteVector

import scala.language.implicitConversions
import scala.concurrent.ExecutionContext.global
import scala.util.Random

class KademliaSimulationSpec extends WordSpec with Matchers {

  implicit val shift: ContextShift[IO] = IO.contextShift(global)
  implicit val timer: Timer[IO] = IO.timer(global)

  implicit def key(i: Long): Key =
    Key.fromBytes.unsafe(
      Stream.continually(ByteVector.fromLong(i).toArray).flatten.take(Key.Length).toArray
    )

  def keyToLong(k: Key): Long =
    k.value.take(java.lang.Long.BYTES).toLong()

  implicit val sk: Show[Key] =
    (k: Key) ⇒ Console.CYAN + java.lang.Long.toBinaryString(keyToLong(k)).reverse.padTo(64, '-').reverse + Console.RESET

  implicit val sn: Show[Node[Long]] =
    (n: Node[Long]) ⇒ s"Node(${n.key.show}, ${n.contact})"

  private def now = Instant.now()

  "kademlia simulation" should {
    // Kademlia's K
    val K = 16
    // Number of nodes in simulation
    val N = 20
    // Size of probe
    val P = 5

    val seed = 1000004

    "launch with 200 nodes" in {
      implicit val ctx = Context.init("spec", "launch")
      implicit val log = ChainLog.forCtx[IO]

      val random = new Random(seed)

      lazy val nodes: Map[Long, Kademlia[IO, Long]] =
        TestKademlia.simulationIO(K, N, keyToLong, key(random.nextLong()), 3)

      //println("\n\n\n======================================\n\n\n")

      random.shuffle(nodes.toVector).take(P).foreach {
        case (i, ki) ⇒
          random.shuffle(nodes.values).take(P).filterNot(_.nodeKey === ki.nodeKey).foreach { kj ⇒
            implicit val ctx = Context.init(kj.nodeKey.asBase58)
            val log = ChainLog.forCtx[IO]

            val neighbors = kj.lookupIterative(i, K)(log).unsafeRunSync()

            //println(log.mkStringF().unsafeRunSync())

            neighbors.size shouldBe (K min N)
            neighbors.map(_.contact) should contain(i)
          }
      }
    }

    // pre-building a network the same way as it's done above
    val random = new Random(seed)

    val nodes = {
      implicit val ctx = Context.init("spec", "simulationIO")
      import ChainLog.forCtx
      TestKademlia.simulationIO[Long](K, N, keyToLong, random.nextLong(), joinPeers = 2)
    }

    "callIterative: make no more requests then limit in callIterative" in {
      implicit val ctx = Context.init("spec", "callIt-limit", Log.Debug)

      implicit val log = new PrintlnLog[IO](ctx)

      val kad = nodes.drop(N / 4).head._2

      val counter = Ref[IO].of(0).unsafeRunSync()
      val increment =
        counter.update(_ + 1)

      kad
        .callIterative[String, Unit](
          nodes.last._1,
          _ ⇒ EitherT(increment.map[Either[String, Unit]](_ ⇒ Left("This should never succeed"))),
          K min P,
          K max P max (N / 3),
          isIdempotentFn = false
        )
        .unsafeRunSync() shouldBe empty

      counter.get.unsafeRunSync() shouldBe (K max P max (N / 3))
    }

    "callIterative: make no less requests then num+parallelism in idempotent callIterative" in {
      implicit val ctx = Context.init("spec", "callIt-idempotence")
      import ChainLog.forCtx

      val kad = nodes.drop(N / 3).head._2

      val counter = MVar[IO].of(0).unsafeRunSync()
      val increment =
        for {
          v ← counter.take
          _ ← counter.put(v + 1)
        } yield ()

      val numToFind = K min P

      kad
        .callIterative[Throwable, Unit](
          nodes.last._1,
          _ ⇒ EitherT.right[Throwable](increment),
          K min P,
          K max P max (N / 3),
          isIdempotentFn = true
        )
        .unsafeRunSync()
        .size shouldBe counter.read.unsafeRunSync()

      counter.read.unsafeRunSync() should be <= (numToFind + 3)
      counter.read.unsafeRunSync() should be >= numToFind
    }

    "callIterative: make num calls in non-idempotent callIterative" in {
      implicit val ctx = Context.init("spec", "callIt-calls")
      import ChainLog.forCtx

      val kad = nodes.drop(N / 3).head._2

      val counter = MVar[IO].of(0).unsafeRunSync()
      val increment =
        for {
          v ← counter.take
          _ ← counter.put(v + 1)
        } yield ()
      val numToFind = K min P

      kad
        .callIterative[Throwable, Unit](
          nodes.last._1,
          _ ⇒ EitherT.right[Throwable](increment),
          K min P,
          K max P max (N / 3),
          isIdempotentFn = false
        )
        .unsafeRunSync()
        .size shouldBe numToFind

      counter.read.unsafeRunSync() shouldBe numToFind
    }
  }
}
