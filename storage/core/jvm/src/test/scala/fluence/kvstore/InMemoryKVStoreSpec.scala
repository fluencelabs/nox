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

import java.nio.ByteBuffer

import cats.effect.IO
import cats.~>
import monix.execution.Scheduler.Implicits.global
import monix.reactive.Observable
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.{Matchers, WordSpec}

import scala.concurrent.duration._
import scala.language.{higherKinds, implicitConversions}

class InMemoryKVStoreSpec extends WordSpec with Matchers with ScalaFutures {

  import fluence.kvstore.KVStore._

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

      store.get(key1).run[IO].value.unsafeRunSync().right.get shouldBe None
      store.get(key1).runF[IO].unsafeRunSync() shouldBe None
      store.get(key1).runEither[IO].unsafeRunSync().right.get shouldBe None
      store.get(key1).runUnsafe() shouldBe None

      store.put(key1, val1).runUnsafe() shouldBe ()

      store.get(key1).run[IO].value.unsafeRunSync().right.get.get shouldBe val1
      store.get(key1).runF[IO].unsafeRunSync().get shouldBe val1
      store.get(key1).runEither[IO].unsafeRunSync().right.get.get shouldBe val1
      store.get(key1).runUnsafe().get shouldBe val1

    }

    "perform all Traverse operations correctly" in {

      val store = InMemoryKVStore[String, String]

      val key1 = "key1"
      val val1 = "val1"
      val key2 = "key2"
      val val2 = "val2"

      store.traverse.run[Observable].toListL.runAsync.futureValue shouldBe empty
      store.traverse.runUnsafe shouldBe empty

      store.put(key1, val1).runUnsafe() shouldBe ()
      store.put(key2, val2).runUnsafe() shouldBe ()

      val expectedPairs = List(key1 → val1, key2 → val2)
      store.traverse.run[Observable].toListL.runAsync.futureValue should contain theSameElementsAs expectedPairs
      store.traverse.runUnsafe.toList should contain theSameElementsAs expectedPairs

    }

    "perform all Put operations correctly" in {

      val store = InMemoryKVStore[String, String]

      val key1 = "key1"
      val val1 = "val1"
      val key2 = "key2"
      val val2 = "val2"
      val key3 = "key3"
      val val3 = "val3"
      val key4 = "key4"
      val val4 = "val4"

      store.put(key1, val1).run[IO].value.unsafeRunSync().right.get shouldBe ()
      store.put(key2, val2).runEither[IO].unsafeRunSync().right.get shouldBe ()
      store.put(key3, val3).runF[IO].unsafeRunSync() shouldBe ()
      store.put(key4, val4).runUnsafe() shouldBe ()

      val expectedPairs = Seq(key1 → val1, key2 → val2, key3 → val3, key4 → val4)
      store.traverse.runUnsafe.toList should contain theSameElementsAs expectedPairs

    }

    "perform all Remove operations correctly" in {

      val store = InMemoryKVStore[String, String]

      val key1 = "key1"
      val val1 = "val1"
      val key2 = "key2"
      val val2 = "val2"
      val key3 = "key3"
      val val3 = "val3"
      val key4 = "key4"
      val val4 = "val4"

      store.remove(key1).run[IO].value.unsafeRunSync().right.get shouldBe ()
      store.remove(key3).runEither[IO].unsafeRunSync().right.get shouldBe ()
      store.remove(key2).runF[IO].unsafeRunSync() shouldBe ()
      store.remove(key4).runUnsafe() shouldBe ()

      store.put(key1, val1).run[IO].value.unsafeRunSync().right.get shouldBe ()
      store.put(key2, val2).runEither[IO].unsafeRunSync().right.get shouldBe ()
      store.put(key3, val3).runF[IO].unsafeRunSync() shouldBe ()
      store.put(key4, val4).runUnsafe() shouldBe ()

      store.remove(key1).run[IO].value.unsafeRunSync().right.get shouldBe ()
      store.remove(key3).runEither[IO].unsafeRunSync().right.get shouldBe ()
      store.remove(key2).runF[IO].unsafeRunSync() shouldBe ()
      store.remove(key4).runUnsafe() shouldBe ()

      store.traverse.runUnsafe.toList shouldBe empty

    }

    "performs all operations correctly" in {

      val store = InMemoryKVStore[ByteBuffer, Array[Byte]]

      val key1 = "key1".getBytes()
      val val1 = "val1".getBytes()
      val key2 = "key2".getBytes()
      val val2 = "val2".getBytes()
      val newVal2 = "new val2".getBytes()

      // check write and read

      store.get(key1).runUnsafe() shouldBe None
      store.put(key1, val1).runUnsafe() shouldBe ()
      store.get(key1).runUnsafe().get shouldBe val1

      // check update

      store.put(key2, val2).runUnsafe() shouldBe ()
      store.get(key2).runUnsafe().get shouldBe val2
      store.put(key2, newVal2).runUnsafe() shouldBe ()
      store.get(key2).runUnsafe().get shouldBe newVal2

      // check delete

      store.get(key1).runUnsafe().get shouldBe val1
      store.remove(key1).runUnsafe() shouldBe ()
      store.get(key1).runUnsafe() shouldBe None

      // check traverse

      val manyPairs: Seq[(Key, Value)] = 1 to 100 map { n ⇒
        s"key$n".getBytes() → s"val$n".getBytes()
      }
      val inserts = manyPairs.map { case (k, v) ⇒ store.put(k, v).runUnsafe() }
      inserts should have size 100

      val traverseResult = store.traverse.run[Observable].toListL.runSyncUnsafe(1.seconds)

      bytesToStr(traverseResult.map {
        case (bb, v) ⇒ bb.array() -> v
      }) should contain theSameElementsAs bytesToStr(manyPairs)

    }

    "performs all operations correctly with snapshot" in {

      val store = InMemoryKVStore.withSnapshots[ByteBuffer, Array[Byte]]

      val key1 = "key1".getBytes()
      val val1 = "val1".getBytes()
      val key2 = "key2".getBytes()
      val val2 = "val2".getBytes()
      val newVal2 = "new val2".getBytes()

      store.put(key1, val1).runUnsafe() shouldBe ()
      store.put(key2, val2).runUnsafe() shouldBe ()

      // check delete

      val storeSnapshot1 = store.createSnapshot[IO]().unsafeRunSync()
      storeSnapshot1.get(key1).runUnsafe().get shouldBe val1

      store.get(key1).runUnsafe().get shouldBe val1
      store.remove(key1).runUnsafe() shouldBe ()
      store.get(key1).runUnsafe() shouldBe None
      storeSnapshot1.get(key1).runUnsafe().get shouldBe val1

      // check traverse

      val manyPairs: Seq[(Key, Value)] = 1 to 100 map { n ⇒
        s"key$n".getBytes() → s"val$n".getBytes()
      }
      val inserts = manyPairs.map { case (k, v) ⇒ store.put(k, v).runUnsafe() }
      inserts should have size 100

      val traverseResult = store.traverse.runUnsafe.toList

      bytesToStr(traverseResult.map {
        case (bb, v) ⇒ bb.array() -> v
      }) should contain theSameElementsAs bytesToStr(manyPairs)

      // take snapshot and remove all element in store
      val storeSnapshot2 = store.createSnapshot[IO]().unsafeRunSync()

      traverseResult.foreach { case (k, _) ⇒ store.remove(k).runUnsafe() }
      val traverseResult2 = store.traverse.runUnsafe.toList
      traverseResult2 shouldBe empty

      val traverseResult3 = storeSnapshot2.traverse.runUnsafe.toList

      bytesToStr(traverseResult3.map {
        case (bb, v) ⇒ bb.array() -> v
      }) should contain theSameElementsAs bytesToStr(manyPairs)

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
