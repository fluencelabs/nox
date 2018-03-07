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
import fluence.kad.protocol.{ KademliaRpc, Key, Node }
import org.scalatest.{ Matchers, WordSpec }
import monix.eval.Coeval

import scala.concurrent.duration._

class BucketSpec extends WordSpec with Matchers {

  "kademlia bucket" should {

    val pingDuration = Duration.Undefined

    type C = Int
    type F[A] = StateT[Coeval, Bucket[C], A]

    implicit class BucketOps(state: Bucket[C]) extends Bucket.WriteOps[F, C] {
      override protected def run[T](bucketId: Int, mod: StateT[F, Bucket[C], T]): F[T] =
        mod.run(state).flatMap{
          case (s, v) ⇒
            StateT.set[Coeval, Bucket[C]](s).map(_ ⇒ v)
        }

      override def read(bucketId: Int): Bucket[C] =
        state
    }

    "update contacts" in {

      val b0 = Bucket[C](2)
      val k0 = Key.fromBytes[Coeval](Array.fill(Key.Length)(1: Byte)).value
      val k1 = Key.fromBytes[Coeval](Array.fill(Key.Length)(2: Byte)).value
      val k2 = Key.fromBytes[Coeval](Array.fill(Key.Length)(3: Byte)).value

      val failRPC = (_: C) ⇒ new KademliaRpc[F, C] {
        override def ping() = StateT.liftF(Coeval.raiseError(new NoSuchElementException))

        override def lookup(key: Key, numberOfNodes: Int) = ???

        override def lookupAway(key: Key, moveAwayFrom: Key, numberOfNodes: C) = ???
      }

      val successRPC = (c: C) ⇒ new KademliaRpc[F, C] {
        override def ping() = StateT.liftF(Coeval(Node(Key.fromBytes[Coeval](Array.fill(Key.Length)(c.toByte)).value, Instant.now(), c)))

        override def lookup(key: Key, numberOfNodes: Int) = ???
        override def lookupAway(key: Key, moveAwayFrom: Key, numberOfNodes: C) = ???
      }

      // By default, bucket is empty
      b0.find(k0) shouldBe empty

      // Adding one contact, bucket should save it
      val (b1, true) = b0.update(0, Node[C](k0, Instant.now(), 1), failRPC, pingDuration).run(b0).value

      b1.find(k0) shouldBe defined

      // Adding second contact, bucket should save it
      val (b2, true) = b1.update(0, Node(k1, Instant.now(), 2), failRPC, pingDuration).run(b1).value

      b2.find(k0) shouldBe defined
      b2.find(k1) shouldBe defined

      // Adding third contact, bucket is full, so if the least recent item is not responding, drop it
      val (b3, true) = b2.update(0, Node(k2, Instant.now(), 3), failRPC, pingDuration).run(b2).value

      b3.find(k0) shouldBe empty
      b3.find(k1) shouldBe defined
      b3.find(k2) shouldBe defined

      // Adding third contact, bucket is full, so if the least recent item is responding, drop the new contact
      val (b4, false) = b2.update(0, Node(k2, Instant.now(), 3), successRPC, pingDuration).run(b2).value

      b4.find(k0) shouldBe defined
      b4.find(k1) shouldBe defined
      b4.find(k2) shouldBe empty
    }

  }

}
