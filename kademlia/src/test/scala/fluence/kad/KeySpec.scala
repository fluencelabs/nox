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

package fluence.kad

import java.nio.ByteBuffer

import cats.kernel.Monoid
import cats.syntax.monoid._
import cats.syntax.order._
import fluence.kad.protocol.Key
import org.scalatest.Matchers
import org.scalatest.wordspec.AnyWordSpec
import scodec.bits.ByteVector

import scala.language.implicitConversions

class KeySpec extends AnyWordSpec with Matchers {

  "kademlia key" should {

    implicit def key(i: Long): Key =
      Key.fromBytes.unsafe(Array.concat(Array.ofDim[Byte](Key.Length - java.lang.Long.BYTES), {
        ByteVector.fromLong(i).toArray
      }))

    implicit def toLong(k: Key): Long = {
      val buffer = ByteBuffer.allocate(java.lang.Long.BYTES)
      buffer.put(k.id.takeRight(java.lang.Long.BYTES))
      buffer.flip()
      buffer.getLong()
    }

    "count leading zeros" in {
      Monoid[Key].empty.zerosPrefixLen shouldBe Key.BitLength
      Key.fromBytes.unsafe(Array.fill(Key.Length)(81: Byte)).zerosPrefixLen shouldBe 1
      Key.fromBytes.unsafe(Array.fill(Key.Length)(1: Byte)).zerosPrefixLen shouldBe 7
      Key.fromBytes
        .unsafe(Array.concat(Array.ofDim[Byte](1), Array.fill(Key.Length - 1)(81: Byte)))
        .zerosPrefixLen shouldBe 9

      val k = (5653605169450630095L: Key) |+| (-4904931527322633638L: Key)

      (k !== Monoid[Key].empty) shouldBe true

      k.zerosPrefixLen shouldBe 96
    }

    "sort keys" in {

      Key.fromBytes.unsafe(Array.fill(Key.Length)(81: Byte)).compare(Monoid[Key].empty) should be > 0
      Key.fromBytes
        .unsafe(Array.fill(Key.Length)(31: Byte))
        .compare(Key.fromBytes.unsafe(Array.fill(Key.Length)(82: Byte))) should be < 0

    }

    "randomize only suffix" in {
      val k = Key.fromBytes.unsafe(Array.fill(Key.Length)(81: Byte))

      k.randomDistantKey(Key.BitLength) shouldBe k

      (k.randomDistantKey(Key.BitLength / 2) |+| k).zerosPrefixLen should be >= (Key.BitLength / 2)
    }

  }

}
