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

package fluence.storage.rocksdb

import cats.instances.try_._
import com.typesafe.config.ConfigFactory
import monix.eval.Task
import monix.execution.{ ExecutionModel, Scheduler }
import org.mockito.ArgumentMatchers.any
import org.mockito.Mockito.{ times, verify }
import org.mockito.{ ArgumentMatchers, Mockito }
import org.rocksdb._
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.mockito.MockitoSugar
import org.scalatest.time.{ Milliseconds, Seconds, Span }
import org.scalatest.{ BeforeAndAfterAll, Matchers, WordSpec }

import scala.reflect.io.Path
import scala.util.Try

class RocksDbStoreSpec extends WordSpec with Matchers with BeforeAndAfterAll with MockitoSugar with ScalaFutures {
  override implicit def patienceConfig: PatienceConfig = PatienceConfig(Span(1, Seconds), Span(250, Milliseconds))
  implicit val scheduler: Scheduler = Scheduler(ExecutionModel.AlwaysAsyncExecution)

  private val conf = RocksDbConf.read[Try](ConfigFactory.load()).get
  assert(conf.dataDir.startsWith(System.getProperty("java.io.tmpdir")))

  "RocksDbStore" should {
    "performs all operations correctly" in {
      import RocksDbStore._

      runRocksDb("RocksDbStoreSpec.test1") { store ⇒

        val key1 = "key1".getBytes()
        val val1 = "val1".getBytes()
        val key2 = "key2".getBytes()
        val val2 = "val2".getBytes()
        val newVal2 = "new val2".getBytes()

        // check write and read

        val case1Result = Task.sequence(Seq(
          store.get(key1).attempt.map(_.toOption),
          store.put(key1, val1),
          store.get(key1)
        )).runAsync.futureValue

        case1Result should contain theSameElementsInOrderAs Seq(None, (), val1)

        // check update

        val case2Result = Task.sequence(Seq(
          store.put(key2, val2),
          store.get(key2),
          store.put(key2, newVal2),
          store.get(key2)
        )).runAsync.futureValue

        case2Result should contain theSameElementsInOrderAs Seq((), val2, (), newVal2)

        // check delete

        val case3Result = Task.sequence(Seq(
          store.get(key1),
          store.remove(key1),
          store.get(key1).attempt.map(_.toOption)
        )).runAsync.futureValue

        case3Result should contain theSameElementsInOrderAs Seq(val1, (), None)

        // check traverse

        val manyPairs: Seq[(Key, Value)] = 1 to 100 map { n ⇒ s"key$n".getBytes() → s"val$n".getBytes() }
        val inserts = manyPairs.map { case (k, v) ⇒ store.put(k, v) }

        val case4 = Task.sequence(inserts).flatMap(_ ⇒ store.traverse().toListL).runAsync

        val traverseResult = case4.futureValue
        bytesToStr(traverseResult) should contain theSameElementsAs bytesToStr(manyPairs)

      }

    }
  }

  "putting to database" should {
    "be always single-threaded" in {

      runRocksDb("RocksDbStoreSpec.test2") { store ⇒
        // execute 100 concurrent put to database
        // if putting will be concurrent, then RocksDb raise an Exception
        val batchInsert = 1 to 100 map { n ⇒
          store.put(s"key$n".getBytes(), s"value$n".getBytes())
        }

        // make sure that all entries are in DB
        Task.gather(batchInsert).runAsync.futureValue

        store.traverse()
          .toListL
          .runAsync
          .futureValue should have size 100
      }
    }
  }

  "traverse" should {
    "take snapshot" when {
      "client starts reading" in {
        implicit val patience: PatienceConfig = PatienceConfig(timeout =  Span(3, Seconds), interval = Span(500, Milliseconds))

        val db = mock[RocksDB]
        val options = mock[Options]
        val snapshot = mock[Snapshot]
        Mockito.when(db.getSnapshot).thenReturn(snapshot)
        Mockito.when(db.newIterator(any(classOf[ReadOptions]))).thenReturn(createTestRocksIterator(5))

        val store = new RocksDbStore("", db, options)

        try {
          val stream = store.traverse()

          verify(db, times(0)).getSnapshot
          verify(db, times(0)).newIterator(any[ReadOptions])

          stream.foreach(_ ⇒ ()).futureValue

          verify(db, times(1)).getSnapshot
          verify(db, times(1)).newIterator(ArgumentMatchers.any[ReadOptions])
          verify(snapshot, times(1)).close()
        } finally {
          store.close()
        }

        verify(db, times(1)).close()
        verify(options, times(1)).close()
      }
    }
  }

  private def runRocksDb(name: String)(action: RocksDbStore ⇒ Unit): Unit = {
    val store = RocksDbStore(name, conf).get
    try action(store) finally store.close()
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

  override protected def afterAll(): Unit = {
    Path(conf.dataDir).deleteRecursively()
  }
}
