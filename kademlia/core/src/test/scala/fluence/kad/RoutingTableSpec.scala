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

import cats.{~>, Applicative, Monad, Parallel}
import cats.data.StateT
import cats.effect.{IO, LiftIO}
import fluence.kad.protocol.{KademliaRpc, Key, Node}
import monix.eval.Coeval
import monix.execution.atomic.Atomic
import org.scalatest.{Matchers, WordSpec}
import scodec.bits.ByteVector

import scala.concurrent.duration._
import scala.language.implicitConversions

class RoutingTableSpec extends WordSpec with Matchers {
  implicit def key(i: Long): Key =
    Key.fromBytes.unsafe(Array.concat(Array.ofDim[Byte](Key.Length - java.lang.Long.BYTES), {
      ByteVector.fromLong(i).toArray
    }))

  implicit def toLong(k: Key): Long = {
    k.value.toLong()
  }

  implicit object liftCoeval extends LiftIO[Coeval] {
    override def liftIO[A](ioa: IO[A]): Coeval[A] = Coeval(ioa.unsafeRunSync())
  }

  private val pingDuration = Duration.Undefined

  private def now = Instant.now()

  "kademlia routing table (non-iterative)" should {

    implicit val par: Parallel[Coeval, Coeval] = new Parallel[Coeval, Coeval] {
      override def applicative = Applicative[Coeval]

      override def monad = Monad[Coeval]

      override def sequential: Coeval ~> Coeval = new (Coeval ~> Coeval) {
        override def apply[A](fa: Coeval[A]) = fa
      }

      override def parallel = sequential
    }

    val failLocalRPC = (_: Long) ⇒
      new KademliaRpc[Long] {
        override def ping() = IO.raiseError(new NoSuchElementException)

        override def lookup(key: Key, numberOfNodes: Int) = ???
        override def lookupAway(key: Key, moveAwayFrom: Key, numberOfNodes: Int) = ???
    }

    val successLocalRPC = (c: Long) ⇒
      new KademliaRpc[Long] {
        override def ping() = IO(Node(c, now, c))

        override def lookup(key: Key, numberOfNodes: Int) = ???
        override def lookupAway(key: Key, moveAwayFrom: Key, numberOfNodes: Int) = ???
    }

    val checkNode: Node[Long] ⇒ IO[Boolean] = _ ⇒ IO(true)

    def bucketOps(maxBucketSize: Int): Bucket.WriteOps[Coeval, Long] =
      new Bucket.WriteOps[Coeval, Long] {
        private val buckets = collection.mutable.Map.empty[Int, Bucket[Long]]

        override protected def run[T](bucketId: Int, mod: StateT[Coeval, Bucket[Long], T]) =
          mod.run(read(bucketId)).map {
            case (b, v) ⇒
              buckets(bucketId) = b
              v
          }

        override def read(bucketId: Int) =
          buckets.getOrElseUpdate(bucketId, Bucket(maxBucketSize))

        override def toString: String =
          buckets.toString()
      }

    def siblingsOps(nodeId: Key, maxSiblingsSize: Int): Siblings.WriteOps[Coeval, Long] =
      new Siblings.WriteOps[Coeval, Long] {
        private val state = Atomic(Siblings[Long](nodeId, maxSiblingsSize))

        override protected def run[T](mod: StateT[Coeval, Siblings[Long], T]) =
          mod.run(read).map {
            case (s, v) ⇒
              state.set(s)
              v
          }

        override def read =
          state.get

        override def toString: String =
          state.get.toString
      }

    "not fail when requesting its own key" in {
      val nodeId: Key = 0l
      implicit val bo = bucketOps(2)
      implicit val so = siblingsOps(nodeId, 2)
      val rt = new RoutingTable[Long](nodeId)

      rt.find(0l) shouldBe empty
      rt.lookup(0l) shouldBe empty
    }

    "find nodes correctly" in {

      val nodeId: Key = 0l
      implicit val bo = bucketOps(2)
      implicit val so = siblingsOps(nodeId, 2)
      val rt = new RoutingTable[Long](nodeId)

      (1l to 5l).foreach { i ⇒
        rt.update[Coeval](Node(i, now, i), failLocalRPC, pingDuration, checkNode).run
        (1l to i).foreach { n ⇒
          rt.find(n) shouldBe defined
        }
      }

      rt.find(4l) shouldBe defined

      rt.update[Coeval](Node(6l, now, 6l), failLocalRPC, pingDuration, checkNode).value shouldBe true

      rt.find(4l) shouldBe empty
      rt.find(6l) shouldBe defined

      rt.update[Coeval](Node(4l, now, 4l), successLocalRPC, pingDuration, checkNode).value shouldBe false

      rt.find(4l) shouldBe empty
      rt.find(6l) shouldBe defined

      rt.update[Coeval](Node(4l, now, 4l), failLocalRPC, pingDuration, checkNode).value shouldBe true

      rt.find(4l) shouldBe defined
      rt.find(6l) shouldBe empty

      so.read.nodes.toList.map(_.contact) shouldBe List(1l, 2l)

    }

    "lookup nodes correctly" in {
      val nodeId: Key = 0l
      implicit val bo = bucketOps(2)
      implicit val so = siblingsOps(nodeId, 10)
      val rt = new RoutingTable[Long](nodeId)

      (1l to 10l).foreach { i ⇒
        rt.update[Coeval](Node(i, now, i), successLocalRPC, pingDuration, checkNode).run
      }

      val nbs10 = rt.lookup(100l)
      nbs10.size should be >= 7

      (1l to 127l).foreach { i ⇒
        rt.update[Coeval](Node(i, now, i), successLocalRPC, pingDuration, checkNode).run
      }

      (1l to 127l).foreach { i ⇒
        rt.lookup(i).size should be >= 10
      }

      so.read.nodes.toList.map(_.contact) shouldBe (1l to 10l).toList
    }
  }
}
