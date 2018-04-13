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

package fluence.storage

import java.nio.ByteBuffer

import cats.effect.IO
import cats.{~>, Id}
import monix.execution.Scheduler.Implicits.global
import monix.reactive.Observable
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.{Matchers, WordSpec}

import scala.concurrent.duration._
import scala.language.{higherKinds, implicitConversions}

class InMemoryKVStoreSpec extends WordSpec with Matchers with ScalaFutures {

  import fluence.storage.InMemoryKVStore._

  type Key = Array[Byte]
  type Value = Array[Byte]

  implicit def wrapBytes(bytes: Array[Byte]): ByteBuffer = ByteBuffer.wrap(bytes)

  implicit val liftToObservable = new (Iterator ~> Observable) {
    override def apply[A](fa: Iterator[A]): Observable[A] = Observable.fromIterator(fa)
  }

  "InMemoryKVStore" should {

    "perform all Get operations correctly" in {

      val store = InMemoryKVStore[ByteBuffer, Array[Byte]]

      val key1 = "key1".getBytes()
      val val1 = "val1".getBytes()

      store.get.run[Id](key1).value.right.get shouldBe None
      store.get.runF[IO](key1).unsafeRunSync() shouldBe None
      store.get.runEither[Id](key1).right.get shouldBe None
      store.get.runUnsafe(key1) shouldBe None

      store.put[Id](key1, val1).right.get shouldBe ()

      store.get.run[Id](key1).value.right.get.get shouldBe val1
      store.get.runF[IO](key1).unsafeRunSync().get shouldBe val1
      store.get.runEither[Id](key1).right.get.get shouldBe val1
      store.get.runUnsafe(key1).get shouldBe val1

    }

    "perform all Traverse operations correctly" in {

      val store = InMemoryKVStore[String, String]

      val key1 = "key1"
      val val1 = "val1"
      val key2 = "key2"
      val val2 = "val2"

      store.traverse.run[Observable].toListL.runAsync.futureValue shouldBe empty
      store.traverse.runUnsafe shouldBe empty

      store.put[Id](key1, val1).right.get shouldBe ()
      store.put[Id](key2, val2).right.get shouldBe ()

      val expectedPairs = List(key1 → val1, key2 → val2)
      store.traverse.run[Observable].toListL.runAsync.futureValue should contain theSameElementsAs expectedPairs
      store.traverse.runUnsafe.toList should contain theSameElementsAs expectedPairs

    }

    "performs all operations correctly" in {

      val store = InMemoryKVStore[ByteBuffer, Array[Byte]]

      val key1 = "key1".getBytes()
      val val1 = "val1".getBytes()
      val key2 = "key2".getBytes()
      val val2 = "val2".getBytes()
      val newVal2 = "new val2".getBytes()

      // check write and read

      store.get.runUnsafe(key1) shouldBe None
      store.put[Id](key1, val1).right.get shouldBe ()
      store.get.runUnsafe(key1).get shouldBe val1

      // check update

      store.put[Id](key2, val2).right.get shouldBe ()
      store.get.runUnsafe(key2).get shouldBe val2
      store.put[Id](key2, newVal2).right.get shouldBe ()
      store.get.runUnsafe(key2).get shouldBe newVal2

      // check delete

      store.get.runUnsafe(key1).get shouldBe val1
      store.remove[Id](key1).right.get shouldBe ()
      store.get.runUnsafe(key1) shouldBe None

      // check traverse

      val manyPairs: Seq[(Key, Value)] = 1 to 100 map { n ⇒
        s"key$n".getBytes() → s"val$n".getBytes()
      }
      val inserts = manyPairs.map { case (k, v) ⇒ store.put[Id](k, v).right.get }
      inserts should have size 100

      val traverseResult = store.traverse.run[Observable].toListL.runSyncUnsafe(1.seconds)

      bytesToStr(traverseResult.map {
        case (bb, v) ⇒ bb.array() -> v
      }) should contain theSameElementsAs bytesToStr(manyPairs)

    }

    "performs all operations correctly with snapshot" in {
      // todo finish
    }
  }

  private def bytesToStr(bytes: Seq[(Array[Byte], Array[Byte])]): Seq[(String, String)] = {
    bytes.map { case (k, v) ⇒ new String(k) → new String(v) }
  }

  private def check(result: Seq[Any], expected: Seq[Any]): Unit =
    result.map {
      case Some(v) ⇒ v
      case x ⇒ x
    } should contain theSameElementsInOrderAs expected

}
