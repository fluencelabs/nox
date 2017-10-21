package fluence.node.storage.rocksdb

import monix.eval.Task
import monix.execution.schedulers.TestScheduler
import monix.execution.{ ExecutionModel, Scheduler }
import org.mockito.ArgumentMatchers.any
import org.mockito.Mockito.{ times, verify }
import org.mockito.{ ArgumentMatchers, Mockito }
import org.rocksdb._
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.mockito.MockitoSugar
import org.scalatest.{ BeforeAndAfterAll, Matchers, WordSpec }

import scala.concurrent.Await
import scala.concurrent.duration._
import scala.reflect.io.Path

class RocksDbStoreSpec extends WordSpec with Matchers with BeforeAndAfterAll with MockitoSugar with ScalaFutures {

  val testFolder: String = System.getProperty("java.io.tmpdir") + "/RocksDbStoreSpec"
  System.setProperty("fluence.node.storage.root", testFolder)

  "RocksDbStore" should {
    "performs all operations correctly" in {
      import RocksDbStore._

      implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

      val store = RocksDbStore("test2").get

      val key1 = "key1".getBytes()
      val val1 = "val1".getBytes()
      val key2 = "key2".getBytes()
      val val2 = "val2".getBytes()
      val newVal2 = "new val2".getBytes()

      // check write and read

      val case1 = Task.sequence(Seq(
        store.get(key1),
        store.put(key1, val1),
        store.get(key1)
      )).runAsync

      testScheduler.tick(5.seconds)

      val case1Result = case1.futureValue
      case1Result should contain theSameElementsInOrderAs Seq(null, (), val1)

      // check update

      val case2 = Task.sequence(Seq(
        store.put(key2, val2),
        store.get(key2),
        store.put(key2, newVal2),
        store.get(key2)
      )).runAsync

      testScheduler.tick(5.seconds)

      val case2Result = case2.futureValue
      case2Result should contain theSameElementsInOrderAs Seq((), val2, (), newVal2)

      // check delete

      val case3 = Task.sequence(Seq(
        store.get(key1),
        store.remove(key1),
        store.get(key1)
      )).runAsync

      testScheduler.tick(5.seconds)

      val case3Result = case3.futureValue
      case3Result should contain theSameElementsInOrderAs Seq(val1, (), null)

      // check traverse

      val manyPairs: Seq[(Key, Value)] = 1 to 100 map { n ⇒ s"key$n".getBytes() → s"val$n".getBytes() }
      val inserts = manyPairs.map { case (k, v) ⇒ store.put(k, v) }

      val case4 = Task.sequence(inserts).flatMap(_ ⇒ store.traverse().toListL).runAsync

      testScheduler.tick(5.seconds)

      val traverseResult = case4.futureValue
      bytesToStr(traverseResult) should contain theSameElementsAs bytesToStr(manyPairs)

    }
  }

  "putting to database" should {
    "be always single-threaded" in {
      implicit val scheduler: Scheduler = Scheduler(ExecutionModel.AlwaysAsyncExecution)

      val store = RocksDbStore("test1").get

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

  "traverse" should {
    "take snapshot" when {
      "client starts reading" in {

        implicit val scheduler: Scheduler = Scheduler(ExecutionModel.AlwaysAsyncExecution)

        val db = mock[RocksDB]
        val options = mock[Options]
        val snapshot = mock[Snapshot]
        Mockito.when(db.getSnapshot).thenReturn(snapshot)
        Mockito.when(db.newIterator(any(classOf[ReadOptions]))).thenReturn(createTestRocksIterator(5))

        val store = new RocksDbStore("", db, options)

        val stream = store.traverse()

        verify(db, times(0)).getSnapshot
        verify(db, times(0)).newIterator(any[ReadOptions])

        val first = stream.foreach(println)

        Await.ready(first, 1.seconds)

        verify(db, times(1)).getSnapshot
        verify(db, times(1)).newIterator(ArgumentMatchers.any[ReadOptions])
        verify(snapshot, times(1)).close()

        store.close()

        verify(db, times(1)).close()
        verify(options, times(1)).close()
      }
    }
  }

  private def createTestRocksIterator(limit: Int): RocksIterator = {
    new RocksIterator(null.asInstanceOf[RocksDB], 1L) {
      private var cursor = -1
      override def seekToFirst(): Unit = cursor = 0
      override def value(): Array[Byte] = s"val$cursor".getBytes
      override def key(): Array[Byte] = s"key$cursor".getBytes()
      override def next(): Unit = cursor += 1
      override def isValid: Boolean = cursor <= limit
    }

  }

  private def bytesToStr(bytes: Seq[(Array[Byte], Array[Byte])]): Seq[(String, String)] = {
    bytes.map { case (k, v) ⇒ new String(k) → new String(v) }
  }

  override protected def afterAll(): Unit = {
    Path(testFolder).deleteRecursively()
  }
}
