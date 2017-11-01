package fluence.kad

import java.time.Instant

import cats.data.StateT
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
      val k0 = Key(Array.fill(Key.Length)(1: Byte))
      val k1 = Key(Array.fill(Key.Length)(2: Byte))
      val k2 = Key(Array.fill(Key.Length)(3: Byte))

      val failRPC = (_: C) ⇒ new KademliaRPC[F, C] {
        override def ping() = StateT.lift(Coeval.raiseError(new NoSuchElementException))

        override def lookup(key: Key, numberOfNodes: Int) = ???

        override def lookupIterative(key: Key, numberOfNodes: Int) = ???
      }

      val successRPC = (c: C) ⇒ new KademliaRPC[F, C] {
        override def ping() = StateT.lift(Coeval(Node(Key(Array.fill(Key.Length)(c.toByte)), Instant.now(), c)))

        override def lookup(key: Key, numberOfNodes: Int) = ???

        override def lookupIterative(key: Key, numberOfNodes: Int) = ???
      }

      // By default, bucket is empty
      b0.find(k0) should be('empty)

      // Adding one contact, bucket should save it
      val (b1, true) = b0.update(0, Node[C](k0, Instant.now(), 1), failRPC, pingDuration).run(b0).value

      b1.find(k0) should be('defined)

      // Adding second contact, bucket should save it
      val (b2, true) = b1.update(0, Node(k1, Instant.now(), 2), failRPC, pingDuration).run(b1).value

      b2.find(k0) should be('defined)
      b2.find(k1) should be('defined)

      // Adding third contact, bucket is full, so if the least recent item is not responding, drop it
      val (b3, true) = b2.update(0, Node(k2, Instant.now(), 3), failRPC, pingDuration).run(b2).value

      b3.find(k0) should be('empty)
      b3.find(k1) should be('defined)
      b3.find(k2) should be('defined)

      // Adding third contact, bucket is full, so if the least recent item is responding, drop the new contact
      val (b4, false) = b2.update(0, Node(k2, Instant.now(), 3), successRPC, pingDuration).run(b2).value

      b4.find(k0) should be('defined)
      b4.find(k1) should be('defined)
      b4.find(k2) should be('empty)
    }

  }

}
