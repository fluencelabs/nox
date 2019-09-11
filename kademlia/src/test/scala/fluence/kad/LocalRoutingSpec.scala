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

import cats.data.EitherT
import cats.effect.{ContextShift, IO, Timer}
import fluence.kad.contact.ContactAccess
import fluence.kad.protocol.{KademliaRpc, Key, Node}
import fluence.kad.routing.LocalRouting
import fluence.kad.state.RoutingState
import fluence.log.{Log, LogFactory}
import org.scalatest.Matchers
import org.scalatest.wordspec.AnyWordSpec
import scodec.bits.ByteVector

import scala.concurrent.ExecutionContext.global
import scala.concurrent.duration._
import scala.language.implicitConversions

class LocalRoutingSpec extends AnyWordSpec with Matchers {
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

  private val logFactory = LogFactory.forChains[IO]()
  implicit val log: Log[IO] = logFactory.init("local-routing-spec").unsafeRunSync()

  "kademlia routing table (non-iterative)" should {

    val failLocalRPC = (_: Long) ⇒
      new KademliaRpc[IO, Long] {
        override def ping()(implicit log: Log[IO]) =
          EitherT.leftT(KadRemoteError("-", new NoSuchElementException): KadRpcError)

        override def lookup(key: Key, numberOfNodes: Int)(implicit log: Log[IO]) = ???
        override def lookupAway(key: Key, moveAwayFrom: Key, numberOfNodes: Int)(implicit log: Log[IO]) = ???
      }

    val successLocalRPC = (c: Long) ⇒
      new KademliaRpc[IO, Long] {
        override def ping()(implicit log: Log[IO]) =
          EitherT.rightT(Node(c, c))

        override def lookup(key: Key, numberOfNodes: Int)(implicit log: Log[IO]) = ???
        override def lookupAway(key: Key, moveAwayFrom: Key, numberOfNodes: Int)(implicit log: Log[IO]) = ???
      }

    val checkNode: Node[Long] ⇒ IO[Boolean] = _ ⇒ IO(true)

    object failCA {
      implicit val ca: ContactAccess[IO, Long] = new ContactAccess[IO, Long](pingDuration, checkNode, failLocalRPC)
    }

    object successCA {
      implicit val ca: ContactAccess[IO, Long] = new ContactAccess[IO, Long](pingDuration, checkNode, successLocalRPC)
    }

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
        import failCA._
        up.update(Node(i, i)).unsafeRunSync()
        (1L to i).foreach { n ⇒
          rt.find(n).unsafeRunSync() shouldBe defined
        }
      }

      rt.find(4L).unsafeRunSync() shouldBe defined

      {
        import failCA._
        up.update(Node(6L, 6L)).unsafeRunSync().updated.contains(6L) shouldBe true
      }

      rt.find(4L).unsafeRunSync() shouldBe empty
      rt.find(6L).unsafeRunSync() shouldBe defined

      {
        import successCA._
        up.update(Node(4L, 4L))
          .unsafeRunSync()
          .updated
          .contains(4L) shouldBe false
      }

      rt.find(4L).unsafeRunSync() shouldBe empty
      rt.find(6L).unsafeRunSync() shouldBe defined

      {
        import failCA._
        up.update(Node(4L, 4L)).unsafeRunSync().updated.contains(4L) shouldBe true
      }

      rt.find(4L).unsafeRunSync() shouldBe defined
      rt.find(6L).unsafeRunSync() shouldBe empty

      up.siblings.unsafeRunSync().nodes.toList.map(_.contact) shouldBe List(1L, 2L)

    }

    "lookup nodes correctly" in {
      val nodeId: Key = 0L
      val (up, rt) = routing(nodeId, maxSiblingsSize = 10)

      (1L to 10L).foreach { i ⇒
        import successCA._
        up.update(Node(i, i)).unsafeRunSync()
      }

      val nbs10 = rt.lookup(100L, 10).unsafeRunSync()
      nbs10.size should be >= 7

      (1L to 127L).foreach { i ⇒
        import successCA._
        up.update(Node(i, i)).unsafeRunSync()
      }

      (1L to 127L).foreach { i ⇒
        rt.lookup(i, 100).unsafeRunSync().size should be >= 10
      }

      up.siblings.unsafeRunSync().nodes.toList.map(_.contact) shouldBe (1L to 10L).toList
    }

    "update contacts" in {
      val (up, rt) = routing(0L, maxSiblingsSize = 10)

      import successCA._

      up.update(Node(1L, 123)).unsafeRunSync()
      rt.find(1L).unsafeRunSync() shouldBe Some(Node(1L, 123))
      rt.lookup(1L, 2, _.key === (1L: Key)).unsafeRunSync() shouldBe Seq(Node(1L, 123))

      up.update(Node(1L, 321)).unsafeRunSync()
      rt.find(1L).unsafeRunSync() shouldBe Some(Node(1L, 321))
      rt.lookup(1L, 2, _.key === (1L: Key)).unsafeRunSync() shouldBe Seq(Node(1L, 321))
    }
  }
}
