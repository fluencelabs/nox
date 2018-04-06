/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

package fluence.kad

import java.time.Instant

import cats.data.StateT
import cats.effect.{IO, LiftIO}
import fluence.kad.protocol.{KademliaRpc, Key, Node}
import org.scalatest.{Matchers, WordSpec}
import monix.eval.Coeval

import scala.concurrent.duration._

class BucketSpec extends WordSpec with Matchers {

  "kademlia bucket" should {

    type C = Int
    type F[A] = StateT[Coeval, Bucket[C], A]

    implicit object liftCoeval extends LiftIO[Coeval] {
      override def liftIO[A](ioa: IO[A]): Coeval[A] = Coeval(ioa.unsafeRunSync())
    }

    def update(node: Node[Int], rpc: C ⇒ KademliaRpc[C]): F[Boolean] =
      Bucket.update[Coeval, C](node, rpc, Duration.Undefined)

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
            IO(Node(Key.fromBytes.unsafe(Array.fill(Key.Length)(c.toByte)), Instant.now(), c))

          override def lookup(key: Key, numberOfNodes: Int) = ???
          override def lookupAway(key: Key, moveAwayFrom: Key, numberOfNodes: C) = ???
      }

      // By default, bucket is empty
      b0.find(k0) shouldBe empty

      // Adding one contact, bucket should save it
      val (b1, true) = update(Node[C](k0, Instant.now(), 1), failRPC).run(b0).value

      b1.find(k0) shouldBe defined

      // Adding second contact, bucket should save it
      val (b2, true) = update(Node(k1, Instant.now(), 2), failRPC).run(b1).value

      b2.find(k0) shouldBe defined
      b2.find(k1) shouldBe defined

      // Adding third contact, bucket is full, so if the least recent item is not responding, drop it
      val (b3, true) = update(Node(k2, Instant.now(), 3), failRPC).run(b2).value

      b3.find(k0) shouldBe empty
      b3.find(k1) shouldBe defined
      b3.find(k2) shouldBe defined

      // Adding third contact, bucket is full, so if the least recent item is responding, drop the new contact
      val (b4, false) = update(Node(k2, Instant.now(), 3), successRPC).run(b2).value

      b4.find(k0) shouldBe defined
      b4.find(k1) shouldBe defined
      b4.find(k2) shouldBe empty
    }

  }

}
