package fluence.kad

import java.nio.ByteBuffer

import cats.kernel.{ Eq, Monoid }
import org.scalatest.{ Matchers, WordSpec }
import cats.syntax.monoid._
import cats.syntax.order._
import scala.language.implicitConversions

class KeySpec extends WordSpec with Matchers {

  "kademlia key" should {

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

    "have correct XOR monoid" in {

      val id = Key(Array.fill(Key.Length)(81: Byte))
      val eqv = Eq[Key].eqv(_, _) // as we can't simply compare byte arrays

      eqv(Monoid[Key].empty |+| id, id) shouldBe true
      eqv(id |+| Monoid[Key].empty, id) shouldBe true
      eqv(Monoid[Key].empty |+| Key.XorDistanceMonoid.empty, Key.XorDistanceMonoid.empty) shouldBe true
    }

    "count leading zeros" in {
      Monoid[Key].empty.zerosPrefixLen shouldBe Key.BitLength
      Key(Array.fill(Key.Length)(81: Byte)).zerosPrefixLen shouldBe 1
      Key(Array.fill(Key.Length)(1: Byte)).zerosPrefixLen shouldBe 7
      Key(Array.concat(Array.ofDim[Byte](1), Array.fill(Key.Length - 1)(81: Byte))).zerosPrefixLen shouldBe 9

      val k = (5653605169450630095l: Key) |+| (-4904931527322633638l: Key)

      (k !== Monoid[Key].empty) shouldBe true

      k.zerosPrefixLen shouldBe 72
    }

    "sort keys" in {

      Key(Array.fill(Key.Length)(81: Byte)).compare(Monoid[Key].empty) should be.>(0)
      Key(Array.fill(Key.Length)(31: Byte)).compare(Key(Array.fill(Key.Length)(82: Byte))) should be.<(0)

    }

  }

}
