package fluence.kad

import java.nio.ByteBuffer
import java.time.Instant

import cats.data.StateT
import fluence.kad.RoutingTable._
import monix.eval.Coeval
import monix.execution.atomic.Atomic
import org.scalatest.{ Matchers, WordSpec }

import scala.collection.concurrent.TrieMap
import scala.concurrent.duration._
import scala.language.implicitConversions

class RoutingTableSpec extends WordSpec with Matchers {
  implicit def key(i: Long): Key = Key(Array.concat(Array.ofDim[Byte](Key.Length - java.lang.Long.BYTES), {
    val buffer = ByteBuffer.allocate(java.lang.Long.BYTES)
    buffer.putLong(i)
    buffer.array()
  }))

  implicit def toLong(k: Key): Long = {
    val buffer = ByteBuffer.allocate(java.lang.Long.BYTES)
    buffer.put(k.id.takeRight(java.lang.Long.BYTES))
    buffer.flip()
    buffer.getLong()
  }

  private val pingDuration = Duration.Undefined

  private def now = Instant.now()

  "kademlia routing table (non-iterative)" should {

    val failLocalRPC = (_: Long) ⇒ new KademliaRPC[Coeval, Long] {
      override def ping() = Coeval.raiseError(new NoSuchElementException)

      override def lookup(key: Key, numberOfNodes: Int) = ???

      override def lookupIterative(key: Key, numberOfNodes: Int) = ???
    }

    val successLocalRPC = (c: Long) ⇒ new KademliaRPC[Coeval, Long] {
      override def ping() = Coeval(Node(c, now, c))

      override def lookup(key: Key, numberOfNodes: Int) = ???

      override def lookupIterative(key: Key, numberOfNodes: Int) = ???
    }

    def bucketOps(maxBucketSize: Int): Bucket.WriteOps[Coeval, Long] =
      new Bucket.WriteOps[Coeval, Long] {
        private val buckets = TrieMap.empty[Int, Bucket[Long]]

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

      nodeId.find(0l) should be('empty)
      nodeId.lookup(0l) should be('empty)
    }

    "find nodes correctly" in {

      val nodeId: Key = 0l
      implicit val bo = bucketOps(2)
      implicit val so = siblingsOps(nodeId, 2)

      (1l to 5l).foreach { i ⇒
        nodeId.update(Node(i, now, i), failLocalRPC, pingDuration).run
        (1l to i).foreach { n ⇒
          nodeId.find(n) should be('defined)
        }
      }

      nodeId.find(4l) should be('defined)

      nodeId.update(Node(6l, now, 6l), failLocalRPC, pingDuration).value shouldBe true

      nodeId.find(4l) should be('empty)
      nodeId.find(6l) should be('defined)

      nodeId.update(Node(4l, now, 4l), successLocalRPC, pingDuration).value shouldBe false

      nodeId.find(4l) should be('empty)
      nodeId.find(6l) should be('defined)

      nodeId.update(Node(4l, now, 4l), failLocalRPC, pingDuration).value shouldBe true

      nodeId.find(4l) should be('defined)
      nodeId.find(6l) should be('empty)

      so.read.nodes.toList.map(_.contact) shouldBe List(1l, 2l)

    }

    "lookup nodes correctly" in {
      val nodeId: Key = 0l
      implicit val bo = bucketOps(2)
      implicit val so = siblingsOps(nodeId, 10)

      (1l to 10l).foreach {
        i ⇒
          nodeId.update(Node(i, now, i), successLocalRPC, pingDuration).run
      }

      val nbs10 = nodeId.lookup(100l)
      nbs10.size should be >= 7

      (1l to 127l).foreach {
        i ⇒
          nodeId.update(Node(i, now, i), successLocalRPC, pingDuration).run
      }

      (1l to 127l).foreach { i ⇒
        nodeId.lookup(i).size should be >= 10
      }

      so.read.nodes.toList.map(_.contact) shouldBe (1l to 10l).toList
    }
  }
}
