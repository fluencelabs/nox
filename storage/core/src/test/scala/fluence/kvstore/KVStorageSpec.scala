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

package fluence.kvstore

import fluence.codec.PureCodec
import org.scalatest.{Matchers, WordSpec}

class KVStorageSpec extends WordSpec with Matchers {

  "transform" should {
    "wrap origin store with codecs" in {

      implicit val strCodec = PureCodec.liftB[String, Array[Byte]](str ⇒ str.getBytes, bytes ⇒ new String(bytes))
      implicit val longCodec = PureCodec.liftB[Long, String](long ⇒ long.toString, str ⇒ str.toLong)

      val store: ReadWriteKVStore[Long, String] = new TestKVStore[String, Array[Byte]]

      store.put(1L, "test").runUnsafe() shouldBe ()
      store.get(1L).runUnsafe() shouldBe Some("test")
      store.traverse.runUnsafe.toList should contain only 1L → "test"
      store.remove(1L).runUnsafe() shouldBe ()
      store.get(1L).runUnsafe() shouldBe None

    }
  }

}
