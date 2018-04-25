package fluence.kvstore.rocksdb

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

import java.nio.ByteBuffer

import cats.effect.IO
import cats.~>
import com.typesafe.config.ConfigFactory
import fluence.kvstore.rocksdb.ObservableLiftIO._
import fluence.kvstore.rocksdb.RocksDbKVStore.RocksDbSnapshotable
import monix.execution.{ExecutionModel, Scheduler}
import monix.reactive.Observable
import org.rocksdb.{RocksDB, RocksIterator}
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.mockito.MockitoSugar
import org.scalatest.time.{Milliseconds, Seconds, Span}
import org.scalatest.{BeforeAndAfterAll, Matchers, WordSpec}

import scala.language.{higherKinds, implicitConversions}
import scala.reflect.io.Path
import scala.util.Random

class RocksDbKVStoreSpec extends WordSpec with Matchers with BeforeAndAfterAll with MockitoSugar with ScalaFutures {
  override implicit def patienceConfig: PatienceConfig = PatienceConfig(Span(1, Seconds), Span(250, Milliseconds))
  implicit val scheduler: Scheduler = Scheduler(ExecutionModel.AlwaysAsyncExecution)

  implicit def wrapBytes(bytes: Array[Byte]): ByteBuffer = ByteBuffer.wrap(bytes)

  implicit val liftToObservable: Iterator ~> Observable = new (Iterator ~> Observable) {
    override def apply[A](fa: Iterator[A]): Observable[A] = Observable.fromIterator(fa)
  }

  private val config = ConfigFactory.load()
  private val rDBConf = RocksDbConf.read[IO](config).value.unsafeRunSync().right.get
  assert(rDBConf.dataDir.startsWith(System.getProperty("java.io.tmpdir")))

  "InMemoryKVStore" should {

    "perform all Get operations correctly" in {

      val key1 = "key1".getBytes()
      val val1 = "val1".getBytes()

      runRocksDb("test1") { store ⇒
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

    }

    "perform all Traverse operations correctly" in {

      val key1 = "key1".getBytes()
      val val1 = "val1".getBytes()
      val key2 = "key2".getBytes()
      val val2 = "val2".getBytes()

      runRocksDbWithSnapshots("test2") { store ⇒
        val store1 = store.createSnapshot[IO]().unsafeRunSync()
        store1.traverse.run[Observable].toListL.runAsync.futureValue shouldBe empty
        store1.traverse.runUnsafe shouldBe empty

        store.put(key1, val1).runUnsafe() shouldBe ()
        store.put(key2, val2).runUnsafe() shouldBe ()

        val expectedPairs = List(key1 → val1, key2 → val2)

        val store2 = store.createSnapshot[IO]().unsafeRunSync()

        val traverseResult1 = store2.traverse.run[Observable].toListL.runAsync.futureValue
        bytesToStr(traverseResult1) should contain theSameElementsAs bytesToStr(expectedPairs)

        val traverseResult2 = store2.traverse.runUnsafe.toList
        bytesToStr(traverseResult1) should contain theSameElementsAs bytesToStr(expectedPairs)

      }

    }

    "perform all Put operations correctly" in {

      val key1 = "key1".getBytes()
      val val1 = "val1".getBytes()
      val key2 = "key2".getBytes()
      val val2 = "val2".getBytes()
      val key3 = "key3".getBytes()
      val val3 = "val3".getBytes()
      val key4 = "key4".getBytes()
      val val4 = "val4".getBytes()

      runRocksDbWithSnapshots("test3") { store ⇒
        store.put(key1, val1).run[IO].value.unsafeRunSync().right.get shouldBe ()
        store.put(key2, val2).runEither[IO].unsafeRunSync().right.get shouldBe ()
        store.put(key3, val3).runF[IO].unsafeRunSync() shouldBe ()
        store.put(key4, val4).runUnsafe() shouldBe ()

        val expectedPairs = Seq(key1 → val1, key2 → val2, key3 → val3, key4 → val4)
        val traverseResult = store.createSnapshot[IO]().unsafeRunSync().traverse.runUnsafe.toList
        bytesToStr(traverseResult) should contain theSameElementsAs bytesToStr(expectedPairs)

      }

    }

//    "perform all Remove operations correctly" in {
//
//      val store = InMemoryKVStore.withSnapshots[String, String]
//
//      val key1 = "key1"
//      val val1 = "val1"
//      val key2 = "key2"
//      val val2 = "val2"
//      val key3 = "key3"
//      val val3 = "val3"
//      val key4 = "key4"
//      val val4 = "val4"
//
//      store.remove(key1).run[IO].value.unsafeRunSync().right.get shouldBe ()
//      store.remove(key3).runEither[IO].unsafeRunSync().right.get shouldBe ()
//      store.remove(key2).runF[IO].unsafeRunSync() shouldBe ()
//      store.remove(key4).runUnsafe() shouldBe ()
//
//      store.put(key1, val1).run[IO].value.unsafeRunSync().right.get shouldBe ()
//      store.put(key2, val2).runEither[IO].unsafeRunSync().right.get shouldBe ()
//      store.put(key3, val3).runF[IO].unsafeRunSync() shouldBe ()
//      store.put(key4, val4).runUnsafe() shouldBe ()
//
//      store.remove(key1).run[IO].value.unsafeRunSync().right.get shouldBe ()
//      store.remove(key3).runEither[IO].unsafeRunSync().right.get shouldBe ()
//      store.remove(key2).runF[IO].unsafeRunSync() shouldBe ()
//      store.remove(key4).runUnsafe() shouldBe ()
//
//      store.createSnapshot[IO]().unsafeRunSync().traverse.runUnsafe.toList shouldBe empty
//
//    }

//    "performs all operations correctly" in {
//
//      val store = InMemoryKVStore.withSnapshots[ByteBuffer, Array[Byte]]
//
//      val key1 = "key1".getBytes()
//      val val1 = "val1".getBytes()
//      val key2 = "key2".getBytes()
//      val val2 = "val2".getBytes()
//      val newVal2 = "new val2".getBytes()
//
//      // check write and read
//
//      store.get(key1).runUnsafe() shouldBe None
//      store.put(key1, val1).runUnsafe() shouldBe ()
//      store.get(key1).runUnsafe().get shouldBe val1
//
//      // check update
//
//      store.put(key2, val2).runUnsafe() shouldBe ()
//      store.get(key2).runUnsafe().get shouldBe val2
//      store.put(key2, newVal2).runUnsafe() shouldBe ()
//      store.get(key2).runUnsafe().get shouldBe newVal2
//
//      // check delete
//
//      store.get(key1).runUnsafe().get shouldBe val1
//      store.remove(key1).runUnsafe() shouldBe ()
//      store.get(key1).runUnsafe() shouldBe None
//
//      // check traverse
//
//      val manyPairs: Seq[(Key, Value)] = 1 to 100 map { n ⇒
//        s"key$n".getBytes() → s"val$n".getBytes()
//      }
//      val inserts = manyPairs.map { case (k, v) ⇒ store.put(k, v).runUnsafe() }
//      inserts should have size 100
//
//      val traverseResult =
//        store.createSnapshot[IO]().unsafeRunSync().traverse.run[Observable].toListL.runSyncUnsafe(1.seconds)
//
//      bytesToStr(traverseResult.map {
//        case (bb, v) ⇒ bb.array() -> v
//      }) should contain theSameElementsAs bytesToStr(manyPairs)
//
//    }

//    "performs all operations correctly with snapshot" in {
//
//      val store = InMemoryKVStore.withSnapshots[ByteBuffer, Array[Byte]]
//
//      val key1 = "key1".getBytes()
//      val val1 = "val1".getBytes()
//      val key2 = "key2".getBytes()
//      val val2 = "val2".getBytes()
//      val newVal2 = "new val2".getBytes()
//
//      store.put(key1, val1).runUnsafe() shouldBe ()
//      store.put(key2, val2).runUnsafe() shouldBe ()
//
//      // check delete
//
//      val storeSnapshot1 = store.createSnapshot[IO]().unsafeRunSync()
//      storeSnapshot1.get(key1).runUnsafe().get shouldBe val1
//
//      store.get(key1).runUnsafe().get shouldBe val1
//      store.remove(key1).runUnsafe() shouldBe ()
//      store.get(key1).runUnsafe() shouldBe None
//      storeSnapshot1.get(key1).runUnsafe().get shouldBe val1
//
//      // check traverse
//
//      val manyPairs: Seq[(Key, Value)] = 1 to 100 map { n ⇒
//        s"key$n".getBytes() → s"val$n".getBytes()
//      }
//      val inserts = manyPairs.map { case (k, v) ⇒ store.put(k, v).runUnsafe() }
//      inserts should have size 100
//
//      val traverseResult = store.createSnapshot[IO]().map(_.traverse.runUnsafe.toList).unsafeRunSync()
//
//      bytesToStr(traverseResult.map {
//        case (bb, v) ⇒ bb.array() -> v
//      }) should contain theSameElementsAs bytesToStr(manyPairs)
//
//      // take snapshot and remove all element in store
//      val storeSnapshot2 = store.createSnapshot[IO]().unsafeRunSync()
//
//      traverseResult.foreach { case (k, _) ⇒ store.remove(k).runUnsafe() }
//      val traverseResult2 = store.createSnapshot[IO]().map(_.traverse.runUnsafe.toList).unsafeRunSync()
//      traverseResult2 shouldBe empty
//
//      val traverseResult3 = storeSnapshot2.traverse.runUnsafe.toList
//
//      bytesToStr(traverseResult3.map {
//        case (bb, v) ⇒ bb.array() -> v
//      }) should contain theSameElementsAs bytesToStr(manyPairs)
//
//    }

    // todo add RocksBd Specific test
  }

  private implicit def long2Bytes(long: Long): Array[Byte] = {
    ByteBuffer.allocate(java.lang.Long.BYTES).putLong(long).array()
  }

  private def runRocksDb(name: String)(action: RocksDbKVStore ⇒ Unit): Unit = {
    val store = RocksDbKVStore.getFactory().value.apply[IO](makeUnique(name), config).value.unsafeRunSync().right.get
    try action(store)
    finally store.close().unsafeRunSync()
  }

  private def runRocksDbWithSnapshots(name: String)(action: RocksDbKVStore with RocksDbSnapshotable ⇒ Unit): Unit = {
    val store =
      RocksDbKVStore.getFactory().value.withSnapshots[IO](makeUnique(name), config).value.unsafeRunSync().right.get
    try action(store)
    finally store.close().unsafeRunSync()
  }

  private def createTestRocksIterator(limit: Int): RocksIterator = {
    new RocksIterator(null.asInstanceOf[RocksDB], 1L) {
      private var cursor = -1
      override def seekToFirst(): Unit = cursor = 0
      override def value(): Array[Byte] = s"val$cursor".getBytes
      override def key(): Array[Byte] = s"key$cursor".getBytes()
      override def next(): Unit = cursor += 1
      override def isValid: Boolean = cursor <= limit
      override def close(): Unit = ()
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

  override protected def afterAll(): Unit = {
    Path(rDBConf.dataDir).deleteRecursively()
  }

  private def makeUnique(dbName: String) = s"${this.getClass.getSimpleName}_${dbName}_${new Random().nextInt}"

}
