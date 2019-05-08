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

import cats.effect.{ContextShift, IO, Timer}
import fluence.kad.routing.LocalRouting
import fluence.kad.protocol.{KademliaRpc, Key, Node}
import fluence.kad.state.RoutingState
import org.scalatest.{Matchers, WordSpec}
import scodec.bits.ByteVector

import scala.concurrent.duration._
import scala.concurrent.ExecutionContext.global
import scala.language.implicitConversions

class LocalRoutingSpec extends WordSpec with Matchers {
  implicit def key(i: Long): Key =
    Key.fromBytes.unsafe(Array.concat(Array.ofDim[Byte](Key.Length - java.lang.Long.BYTES), {
      ByteVector.fromLong(i).toArray
    }))

  implicit def toLong(k: Key): Long = {
    k.value.toLong()
  }

  private val pingDuration = Duration.Undefined

  implicit val shift: ContextShift[IO] = IO.contextShift(global)
  implicit val timer: Timer[IO] = IO.timer(global)

  "kademlia routing table (non-iterative)" should {

    val failLocalRPC = (_: Long) ⇒
      new KademliaRpc[Long] {
        override def ping() = IO.raiseError(new NoSuchElementException)

        override def lookup(key: Key, numberOfNodes: Int) = ???
        override def lookupAway(key: Key, moveAwayFrom: Key, numberOfNodes: Int) = ???
      }

    val successLocalRPC = (c: Long) ⇒
      new KademliaRpc[Long] {
        override def ping() = IO(Node(c, c))

        override def lookup(key: Key, numberOfNodes: Int) = ???
        override def lookupAway(key: Key, moveAwayFrom: Key, numberOfNodes: Int) = ???
      }

    val checkNode: Node[Long] ⇒ IO[Boolean] = _ ⇒ IO(true)

    def routing(
      nodeId: Key,
      maxSiblingsSize: Int = 2,
      maxBucketSize: Int = 2
    ): (RoutingState[IO, Long], LocalRouting[IO, Long]) =
      RoutingState
        .inMemory[IO, IO.Par, Long](nodeId, maxSiblingsSize, maxBucketSize)
        .map(rs ⇒ rs -> LocalRouting(nodeId, rs.siblings, rs.bucket))
        .unsafeRunSync()

    "not fail when requesting its own key" in {
      val nodeId: Key = 0L

      val (up, rt) = routing(nodeId)

      rt.find(0L).unsafeRunSync() shouldBe empty
      rt.lookup(0L, 1).unsafeRunSync() shouldBe empty
    }

    "find nodes correctly" in {

      val nodeId: Key = 0L
      val (up, rt) = routing(nodeId)

      (1L to 5L).foreach { i ⇒
        up.update(Node(i, i), failLocalRPC, pingDuration, checkNode).unsafeRunSync()
        (1L to i).foreach { n ⇒
          rt.find(n).unsafeRunSync() shouldBe defined
        }
      }

      rt.find(4L).unsafeRunSync() shouldBe defined

      up.update(Node(6L, 6L), failLocalRPC, pingDuration, checkNode).unsafeRunSync().updated.contains(6L) shouldBe true

      rt.find(4L).unsafeRunSync() shouldBe empty
      rt.find(6L).unsafeRunSync() shouldBe defined

      up.update(Node(4L, 4L), successLocalRPC, pingDuration, checkNode)
        .unsafeRunSync()
        .updated
        .contains(4L) shouldBe false

      rt.find(4L).unsafeRunSync() shouldBe empty
      rt.find(6L).unsafeRunSync() shouldBe defined

      up.update(Node(4L, 4L), failLocalRPC, pingDuration, checkNode).unsafeRunSync().updated.contains(4L) shouldBe true

      rt.find(4L).unsafeRunSync() shouldBe defined
      rt.find(6L).unsafeRunSync() shouldBe empty

      up.siblings.unsafeRunSync().nodes.toList.map(_.contact) shouldBe List(1L, 2L)

    }

    "lookup nodes correctly" in {
      val nodeId: Key = 0L
      val (up, rt) = routing(nodeId, maxSiblingsSize = 10)

      (1L to 10L).foreach { i ⇒
        up.update(Node(i, i), successLocalRPC, pingDuration, checkNode).unsafeRunSync()
      }

      val nbs10 = rt.lookup(100L, 10).unsafeRunSync()
      nbs10.size should be >= 7

      (1L to 127L).foreach { i ⇒
        up.update(Node(i, i), successLocalRPC, pingDuration, checkNode).unsafeRunSync()
      }

      (1L to 127L).foreach { i ⇒
        rt.lookup(i, 100).unsafeRunSync().size should be >= 10
      }

      up.siblings.unsafeRunSync().nodes.toList.map(_.contact) shouldBe (1L to 10L).toList
    }
  }
}
