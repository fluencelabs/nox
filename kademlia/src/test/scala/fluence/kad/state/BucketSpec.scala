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

package fluence.kad.state

import cats.data.StateT
import cats.effect.{ContextShift, IO, Timer}
import fluence.kad.protocol.{KademliaRpc, Key, Node}
import org.scalatest.{Matchers, WordSpec}

import scala.concurrent.duration._
import scala.concurrent.ExecutionContext.global

class BucketSpec extends WordSpec with Matchers {

  implicit val shift: ContextShift[IO] = IO.contextShift(global)
  implicit val timer: Timer[IO] = IO.timer(global)

  "kademlia bucket" should {

    type C = Int
    type F[A] = StateT[IO, Bucket[C], A]

    def update(node: Node[Int], rpc: C ⇒ KademliaRpc[C]): F[Boolean] =
      Bucket.update[IO, C](node, rpc, Duration.Undefined).map(_.updated.contains(node.key))

    "update contacts" in {

      val b0 = Bucket[C](2)
      val k0 = Key.fromBytes.unsafe(Array.fill(Key.Length)(1: Byte))
      val k1 = Key.fromBytes.unsafe(Array.fill(Key.Length)(2: Byte))
      val k2 = Key.fromBytes.unsafe(Array.fill(Key.Length)(3: Byte))

      val failRPC = (_: C) ⇒
        new KademliaRpc[C] {
          override def ping() = IO.raiseError(new NoSuchElementException)

          override def lookup(key: Key, numberOfNodes: Int) = ???

          override def lookupAway(key: Key, moveAwayFrom: Key, numberOfNodes: C) = ???
      }

      val successRPC = (c: C) ⇒
        new KademliaRpc[C] {
          override def ping() =
            IO(Node(Key.fromBytes.unsafe(Array.fill(Key.Length)(c.toByte)), c))

          override def lookup(key: Key, numberOfNodes: Int) = ???
          override def lookupAway(key: Key, moveAwayFrom: Key, numberOfNodes: C) = ???
      }

      // By default, bucket is empty
      b0.find(k0) shouldBe empty

      // Adding one contact, bucket should save it
      val (b1, true) = update(Node[C](k0, 1), failRPC).run(b0).unsafeRunSync()

      b1.find(k0) shouldBe defined

      // Adding second contact, bucket should save it
      val (b2, true) = update(Node(k1, 2), failRPC).run(b1).unsafeRunSync()

      b2.find(k0) shouldBe defined
      b2.find(k1) shouldBe defined

      // Adding third contact, bucket is full, so if the least recent item is not responding, drop it
      val (b3, true) = update(Node(k2, 3), failRPC).run(b2).unsafeRunSync()

      b3.find(k0) shouldBe empty
      b3.find(k1) shouldBe defined
      b3.find(k2) shouldBe defined

      // Adding third contact, bucket is full, so if the least recent item is responding, drop the new contact
      val (b4, false) = update(Node(k2, 3), successRPC).run(b2).unsafeRunSync()

      b4.find(k0) shouldBe defined
      b4.find(k1) shouldBe defined
      b4.find(k2) shouldBe empty
    }

  }

}
