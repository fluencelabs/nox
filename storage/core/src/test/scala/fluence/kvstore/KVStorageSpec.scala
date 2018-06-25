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

import cats.Monad
import cats.effect.{IO, LiftIO}
import fluence.codec.PureCodec
import org.scalatest.{Matchers, WordSpec}

import scala.language.higherKinds

class KVStorageSpec extends WordSpec with Matchers {

  "transform" should {
    "wrap origin store into codecs" in {

      implicit val strCodec = PureCodec.liftB[String, Array[Byte]](str ⇒ str.getBytes, bytes ⇒ new String(bytes))
      implicit val longCodec = PureCodec.liftB[Long, String](long ⇒ long.toString, str ⇒ str.toLong)

      val store: ReadWriteKVStore[Long, String] = new TestKVStore[String, Array[Byte]]

      store.put(1L, "test").runUnsafe() shouldBe ()
      store.get(1L).runUnsafe() shouldBe Some("test")
      store.traverse.runUnsafe.toList should contain only 1L → "test"
      store.remove(1L).runUnsafe() shouldBe ()
      store.get(1L).runUnsafe() shouldBe None

    }

    "wrap origin snapshotable store into codecs" in {

      implicit val strCodec = PureCodec.liftB[String, Array[Byte]](str ⇒ str.getBytes, bytes ⇒ new String(bytes))
      implicit val longCodec = PureCodec.liftB[Long, String](long ⇒ long.toString, str ⇒ str.toLong)

      val store: ReadWriteKVStore[Long, String] with Snapshotable[KVStoreRead[Long, String]] =
        new TestKVStore[String, Array[Byte]] with Snapshotable[TestKVStore[String, Array[Byte]]] {
          override def createSnapshot[F[_]: Monad: LiftIO]: F[TestKVStore[String, Array[Byte]]] = {
            IO(new TestKVStore(data.clone())).to[F]
          }
        }

      store.put(1L, "test").runUnsafe() shouldBe ()
      store.get(1L).runUnsafe() shouldBe Some("test")
      store.traverse.runUnsafe.toList should contain only 1L → "test"
      val store1 = store.createSnapshot[IO].unsafeRunSync()
      store.remove(1L).runUnsafe() shouldBe ()
      store.get(1L).runUnsafe() shouldBe None
      store1.get(1L).runUnsafe() shouldBe Some("test")

    }
  }

}
