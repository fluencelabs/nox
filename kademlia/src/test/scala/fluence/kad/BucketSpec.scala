package fluence.kad

import cats.Id
import org.scalatest.{ Matchers, WordSpec }
import cats.instances.try_._

import scala.util.{ Failure, Success, Try }

class BucketSpec extends WordSpec with Matchers {

  "kademlia bucket" should {

    "update contacts" in {

      val b0 = Bucket[Byte](2)
      val k0 = Key(Array.fill(Key.Length)(1: Byte))
      val k1 = Key(Array.fill(Key.Length)(2: Byte))
      val k2 = Key(Array.fill(Key.Length)(3: Byte))

      val failRPC = (_: Byte) => new KademliaRPC[Try, Byte] {
        override def ping() = Failure(new NoSuchElementException)

        override def lookup(key: Key) = ???

        override def lookupIterative(key: Key) = ???
      }

      val successRPC = (c: Byte) => new KademliaRPC[Try, Byte] {
        override def ping() = Success(Node(Key(Array.fill(Key.Length)(c)), c))

        override def lookup(key: Key) = ???

        override def lookupIterative(key: Key) = ???
      }

      // By default, bucket is empty
      Bucket.find[Id, Byte](k0).run(b0)._2 should be('empty)

      // Adding one contact, bucket should save it
      val Success((b1, _)) = Bucket.update[Try, Byte](Node(k0, 1), failRPC).run(b0)

      Bucket.find[Id, Byte](k0).run(b1)._2 should be('defined)

      // Adding second contact, bucket should save it
      val Success((b2, _)) = Bucket.update[Try, Byte](Node(k1, 2), failRPC).run(b1)

      Bucket.find[Id, Byte](k0).run(b2)._2 should be('defined)
      Bucket.find[Id, Byte](k1).run(b2)._2 should be('defined)

      // Adding third contact, bucket is full, so if the least recent item is not responding, drop it
      val Success((b3, _)) = Bucket.update[Try, Byte](Node(k2, 3), failRPC).run(b2)

      Bucket.find[Id, Byte](k0).run(b3)._2 should be('empty)
      Bucket.find[Id, Byte](k1).run(b3)._2 should be('defined)
      Bucket.find[Id, Byte](k2).run(b3)._2 should be('defined)

      // Adding third contact, bucket is full, so if the least recent item is responding, drop the new contact
      val Success((b4, _)) = Bucket.update[Try, Byte](Node(k2, 3), successRPC).run(b2)

      Bucket.find[Id, Byte](k0).run(b4)._2 should be('defined)
      Bucket.find[Id, Byte](k1).run(b4)._2 should be('defined)
      Bucket.find[Id, Byte](k2).run(b4)._2 should be('empty)
    }

  }

}
