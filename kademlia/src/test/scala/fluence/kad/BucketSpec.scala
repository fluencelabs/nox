package fluence.kad

import cats.Id
import org.scalatest.{ Matchers, WordSpec }
import cats.instances.try_._

import scala.util.{ Failure, Success, Try }

class BucketSpec extends WordSpec with Matchers {

  "kademlia bucket" should {

    "update contacts" in {

      val b0 = Bucket(2)
      val k0 = Key(Array.fill(Key.Length)(1: Byte))
      val k1 = Key(Array.fill(Key.Length)(2: Byte))
      val k2 = Key(Array.fill(Key.Length)(3: Byte))

      // By default, bucket is empty
      Bucket.find[Id](k0).run(b0)._2 should be('empty)

      // Adding one contact, bucket should save it
      val Success((b1, _)) = Bucket.update[Try](Contact(k0), _ => Failure(new NoSuchElementException)).run(b0)

      Bucket.find[Id](k0).run(b1)._2 should be('defined)

      // Adding second contact, bucket should save it
      val Success((b2, _)) = Bucket.update[Try](Contact(k1), _ => Failure(new NoSuchElementException)).run(b1)

      Bucket.find[Id](k0).run(b2)._2 should be('defined)
      Bucket.find[Id](k1).run(b2)._2 should be('defined)

      // Adding third contact, bucket is full, so if the least recent item is not responding, drop it
      val Success((b3, _)) = Bucket.update[Try](Contact(k2), _ => Failure(new NoSuchElementException)).run(b2)

      Bucket.find[Id](k0).run(b3)._2 should be('empty)
      Bucket.find[Id](k1).run(b3)._2 should be('defined)
      Bucket.find[Id](k2).run(b3)._2 should be('defined)

      // Adding third contact, bucket is full, so if the least recent item is responding, drop the new contact
      val Success((b4, _)) = Bucket.update[Try](Contact(k2), c => Success(c)).run(b2)

      Bucket.find[Id](k0).run(b4)._2 should be('defined)
      Bucket.find[Id](k1).run(b4)._2 should be('defined)
      Bucket.find[Id](k2).run(b4)._2 should be('empty)
    }

  }

}
