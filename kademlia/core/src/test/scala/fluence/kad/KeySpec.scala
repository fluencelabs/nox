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

import java.nio.ByteBuffer

import cats.kernel.{ Eq, Monoid }
import org.scalatest.{ Matchers, WordSpec }
import cats.syntax.monoid._
import cats.syntax.order._
import fluence.kad.protocol.Key
import monix.eval.Coeval

import scala.language.implicitConversions

class KeySpec extends WordSpec with Matchers {

  "kademlia key" should {

    implicit def key(i: Long): Key =
      Key
        .fromBytes[Coeval](Array.concat(Array.ofDim[Byte](Key.Length - java.lang.Long.BYTES), {
          val buffer = ByteBuffer.allocate(java.lang.Long.BYTES)
          buffer.putLong(i)
          buffer.array()
        }))
        .value

    implicit def toLong(k: Key): Long = {
      val buffer = ByteBuffer.allocate(java.lang.Long.BYTES)
      buffer.put(k.id.takeRight(java.lang.Long.BYTES))
      buffer.flip()
      buffer.getLong()
    }

    "have correct XOR monoid" in {

      val id = Key.fromBytes[Coeval](Array.fill(Key.Length)(81: Byte)).value
      val eqv = Eq[Key].eqv(_, _) // as we can't simply compare byte arrays

      eqv(Monoid[Key].empty |+| id, id) shouldBe true
      eqv(id |+| Monoid[Key].empty, id) shouldBe true
      eqv(Monoid[Key].empty |+| Key.XorDistanceMonoid.empty, Key.XorDistanceMonoid.empty) shouldBe true
    }

    "count leading zeros" in {
      Monoid[Key].empty.zerosPrefixLen shouldBe Key.BitLength
      Key.fromBytes[Coeval](Array.fill(Key.Length)(81: Byte)).value.zerosPrefixLen shouldBe 1
      Key.fromBytes[Coeval](Array.fill(Key.Length)(1: Byte)).value.zerosPrefixLen shouldBe 7
      Key
        .fromBytes[Coeval](Array.concat(Array.ofDim[Byte](1), Array.fill(Key.Length - 1)(81: Byte)))
        .value
        .zerosPrefixLen shouldBe 9

      val k = (5653605169450630095l: Key) |+| (-4904931527322633638l: Key)

      (k !== Monoid[Key].empty) shouldBe true

      k.zerosPrefixLen shouldBe 96
    }

    "sort keys" in {

      Key.fromBytes[Coeval](Array.fill(Key.Length)(81: Byte)).value.compare(Monoid[Key].empty) should be > 0
      Key
        .fromBytes[Coeval](Array.fill(Key.Length)(31: Byte))
        .value
        .compare(Key.fromBytes[Coeval](Array.fill(Key.Length)(82: Byte)).value) should be < 0

    }

  }

}
