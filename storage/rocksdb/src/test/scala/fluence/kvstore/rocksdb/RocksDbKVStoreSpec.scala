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
package fluence.kvstore.rocksdb

import java.nio.ByteBuffer
import java.util.concurrent.Executors

import cats.effect.IO
import cats.~>
import com.typesafe.config.ConfigFactory
import fluence.kvstore.rocksdb.ObservableLiftIO._
import fluence.kvstore.rocksdb.RocksDbKVStore.RocksDbSnapshotable
import fluence.storage.rocksdb.RocksDbStore.{Key, Value}
import monix.eval.Task
import monix.execution.{ExecutionModel, Scheduler}
import monix.reactive.Observable
import org.rocksdb.{RocksDB, RocksIterator}
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.mockito.MockitoSugar
import org.scalatest.time.{Milliseconds, Seconds, Span}
import org.scalatest.{BeforeAndAfterAll, Matchers, WordSpec}

import scala.concurrent.ExecutionContext
import scala.language.{higherKinds, implicitConversions}
import scala.reflect.io.Path
import scala.util.Random

class RocksDbKVStoreSpec extends WordSpec with Matchers with BeforeAndAfterAll with MockitoSugar with ScalaFutures {
  override implicit def patienceConfig: PatienceConfig = PatienceConfig(Span(1, Seconds), Span(250, Milliseconds))
  implicit val scheduler: Scheduler = Scheduler(executionModel = ExecutionModel.AlwaysAsyncExecution)

  implicit def wrapBytes(bytes: Array[Byte]): ByteBuffer = ByteBuffer.wrap(bytes)

  implicit val liftToObservable: Iterator ~> Observable = new (Iterator ~> Observable) {
    override def apply[A](fa: Iterator[A]): Observable[A] = Observable.fromIterator(fa)
  }

  private val config = ConfigFactory.load()
  private val rDBConf = RocksDbConf.read[IO](config).value.unsafeRunSync().right.get
  assert(rDBConf.dataDir.startsWith(System.getProperty("java.io.tmpdir")))

  "RocksDbKVStore" should {

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

      runRocksDb("test2") { store ⇒
        store.traverse.run[Observable].toListL.runAsync.futureValue shouldBe empty
        store.traverse.runUnsafe shouldBe empty

        store.put(key1, val1).runUnsafe() shouldBe ()
        store.put(key2, val2).runUnsafe() shouldBe ()

        val expectedPairs = List(key1 → val1, key2 → val2)

        val traverseResult1 = store.traverse.run[Observable].toListL.runAsync.futureValue
        bytesToStr(traverseResult1) should contain theSameElementsAs bytesToStr(expectedPairs)

        val traverseResult2 = store.traverse.runUnsafe.toList
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

      runRocksDb("test3") { store ⇒
        store.put(key1, val1).run[IO].value.unsafeRunSync().right.get shouldBe ()
        store.put(key2, val2).runEither[IO].unsafeRunSync().right.get shouldBe ()
        store.put(key3, val3).runF[IO].unsafeRunSync() shouldBe ()
        store.put(key4, val4).runUnsafe() shouldBe ()

        val expectedPairs = Seq(key1 → val1, key2 → val2, key3 → val3, key4 → val4)
        val traverseResult = store.traverse.runUnsafe.toList
        bytesToStr(traverseResult) should contain theSameElementsAs bytesToStr(expectedPairs)

      }

    }

    "perform all Remove operations correctly" in {

      val key1 = "key1".getBytes()
      val val1 = "val1".getBytes()
      val key2 = "key2".getBytes()
      val val2 = "val2".getBytes()
      val key3 = "key3".getBytes()
      val val3 = "val3".getBytes()
      val key4 = "key4".getBytes()
      val val4 = "val4".getBytes()

      runRocksDb("test4") { store ⇒
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

    }

    "performs all operations correctly" in {

      val key1 = "key1".getBytes()
      val val1 = "val1".getBytes()
      val key2 = "key2".getBytes()
      val val2 = "val2".getBytes()
      val newVal2 = "new val2".getBytes()

      runRocksDb("test5") { store ⇒
        // check write and read

        val case1Result = Seq(
          store.get(key1),
          store.put(key1, val1),
          store.get(key1)
        ).map(_.runUnsafe())

        check(case1Result, Seq(None, (), val1))

        // check update

        val case2Result = Seq(
          store.put(key2, val2),
          store.get(key2),
          store.put(key2, newVal2),
          store.get(key2)
        ).map(_.runUnsafe())

        check(case2Result, Seq((), val2, (), newVal2))

        // check delete

        val case3Result = Seq(
          store.get(key1),
          store.remove(key1),
          store.get(key1)
        ).map(_.runUnsafe())

        check(case3Result, Seq(val1, (), None))

        // check traverse

        val manyPairs: Seq[(Key, Value)] = Random.shuffle(1 to 100).map { n ⇒
          s"key$n".getBytes() → s"val$n".getBytes()
        }

        manyPairs.foreach { case (k, v) ⇒ store.put(k, v).runUnsafe() }

        val traverseResult = store.traverse.runUnsafe.toList
        bytesToStr(traverseResult) should contain theSameElementsAs bytesToStr(manyPairs)

        val maxKey = store.getMaxKey.runUnsafe()
        maxKey.get shouldBe "key99".getBytes // cause k99 > k100 in bytes representation

      }
    }

    "performs all operations correctly with snapshot" in {

      val key1 = "key1".getBytes()
      val val1 = "val1".getBytes()
      val key2 = "key2".getBytes()
      val val2 = "val2".getBytes()
      val key3 = "key3".getBytes()
      val val3 = "val3".getBytes()

      runRocksDbWithSnapshots("test5") { store ⇒
        store.put(key1, val1).runUnsafe() shouldBe ()
        store.put(key2, val2).runUnsafe() shouldBe ()

        // check get

        val storeSnapshot1 = store.createSnapshot[IO].unsafeRunSync()

        storeSnapshot1.get(key1).runUnsafe().get shouldBe val1
        store.get(key1).runUnsafe().get shouldBe val1

        store.remove(key1).runUnsafe() shouldBe ()

        store.get(key1).runUnsafe() shouldBe None
        storeSnapshot1.get(key1).runUnsafe().get shouldBe val1

        store.put(key3, val3).runUnsafe() shouldBe ()

        store.get(key3).runUnsafe().get shouldBe val3
        storeSnapshot1.get(key3).runUnsafe() shouldBe None

        storeSnapshot1.close().unsafeRunSync()

        // check traverse

        val manyPairs: Seq[(Key, Value)] = 1 to 100 map { n ⇒
          s"key$n".getBytes() → s"val$n".getBytes()
        }
        manyPairs.map { case (k, v) ⇒ store.put(k, v).runUnsafe() }

        // take snapshot and remove all element in store
        val storeSnapshot2 = store.createSnapshot[IO].unsafeRunSync()

        val traverseBeforeDelete = storeSnapshot2.traverse.runUnsafe.toList
        bytesToStr(traverseBeforeDelete) should contain theSameElementsAs bytesToStr(manyPairs)

        // do delete
        traverseBeforeDelete.foreach { case (k, _) ⇒ store.remove(k).runUnsafe() }

        val traverseAfterDeleteWithSnapshot = storeSnapshot2.traverse.runUnsafe.toList
        bytesToStr(traverseAfterDeleteWithSnapshot) should contain theSameElementsAs bytesToStr(manyPairs)
        storeSnapshot2.close().unsafeRunSync()

        val traversAfterDeleteWithoutSnapshot = store.traverse.runUnsafe.toList
        traversAfterDeleteWithoutSnapshot shouldBe empty

      }

    }

    "getMaxKey" should {

      "return KeyNotFound when store is empty" in {
        runRocksDb("test6") { store ⇒
          val maxLongKey = store.getMaxKey.runUnsafe()
          maxLongKey shouldBe None
        }
      }

      "get max key for String" in {
        runRocksDb("test7") { store ⇒
          Random.shuffle(1 to 100).map { n ⇒
            s"key$n".getBytes() → s"val$n".getBytes()
          } foreach { case (k, v) ⇒ store.put(k, v).runUnsafe() }

          val maxLongKey = store.getMaxKey.runUnsafe()
          maxLongKey.get shouldBe "key99".getBytes
        }
      }

      "get max key for Long" in {
        runRocksDb("test8") { store ⇒
          Random
            .shuffle(1 to 100)
            .map { n ⇒
              long2Bytes(n) → s"val$n".getBytes()
            }
            .foreach { case (k, v) ⇒ store.put(k, v).runUnsafe() }

          val maxLongKey = store.getMaxKey.runUnsafe()
          maxLongKey.get shouldBe long2Bytes(100L)
        }
      }

    }

    "perform all concurrent mutation and read correctly" in {

      val pool = ExecutionContext.fromExecutorService(Executors.newFixedThreadPool(32))

      runRocksDbWithSnapshots("test2", pool) { store ⇒
        1 to 500 foreach { n ⇒
          store.put(s"_key$n".getBytes(), s"value$n".getBytes()).runUnsafe()
        }

        val snapshotBefore = store.createSnapshot[IO].unsafeRunSync()

        val batchInsert = 1 to 1000 map { n ⇒
          store.put(s"key$n".getBytes(), s"value$n".getBytes()).runF[Task]
        }

        val batchRewriteInsert = 1 to 1000 map { n ⇒
          store.put(s"key$n".getBytes(), s"new value$n".getBytes()).runF[Task]
        }

        val batchDeleteInsert = 1 to 500 map { n ⇒
          store.remove(s"_key$n".getBytes()).runF[Task]
        }

        val traverseBeforeTask = snapshotBefore.traverse.run[Observable].toListL

        val traverseBefore = Task
          .gatherUnordered(batchInsert ++ batchRewriteInsert ++ batchDeleteInsert ++ Seq(traverseBeforeTask))
          .runAsync
          .futureValue
          .filter(_ != ())
          .map(_.asInstanceOf[Seq[(Key, Value)]])

        traverseBefore.head should have size 500
        snapshotBefore.close().unsafeRunSync()

        store.traverse.runUnsafe.toList should have size 1000
      }
    }

  }

  private implicit def long2Bytes(long: Long): Array[Byte] = {
    ByteBuffer.allocate(java.lang.Long.BYTES).putLong(long).array()
  }

  private def runRocksDb(name: String)(action: RocksDbKVStore ⇒ Unit): Unit = {
    val store = RocksDbKVStore.getFactory().value.apply[IO](makeUnique(name), config).value.unsafeRunSync().right.get
    try action(store)
    finally store.close().unsafeRunSync()
  }

  private def runRocksDbWithSnapshots(name: String, tPool: ExecutionContext = scheduler)(
    action: RocksDbKVStore with RocksDbSnapshotable ⇒ Unit
  ): Unit = {
    val store =
      // @formatter:off
      RocksDbKVStore.getFactory(tPool).value
        .withSnapshots[IO](makeUnique(name), config).value.unsafeRunSync().right.get
    // @formatter:on
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
