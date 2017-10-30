package fluence.btree.binary.index.core

import fluence.btree.binary.kryo.KryoCodec
import fluence.btree.core.BTreeBinaryStore
import fluence.node.storage.InMemoryKVStore
import monix.eval.Task
import monix.execution.ExecutionModel
import monix.execution.schedulers.TestScheduler
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.{ Matchers, WordSpec }

import scala.concurrent.duration._

class BTreeBinaryStoreSpec extends WordSpec with Matchers with ScalaFutures {

  "BTreeBinaryStore" should {
    "performs all operations correctly" in {

      implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

      val store = BTreeBinaryStore[Long, String, Task](InMemoryKVStore(), KryoCodec())

      val node1 = "node1"
      val node1Idx = 2L
      val node2 = "node2"
      val node2new = "node2"
      val node2Idx = 3L

      // check write and read

      val case1 = Task.sequence(Seq(
        store.get(node1Idx),
        store.put(node1Idx, node1),
        store.get(node1Idx)
      )).runAsync

      testScheduler.tick(5.seconds)

      val case1Result = case1.futureValue
      case1Result should contain theSameElementsInOrderAs Seq(null, (), node1)

      // check update

      val case2 = Task.sequence(Seq(
        store.put(node2Idx, node2),
        store.get(node2Idx),
        store.put(node2Idx, node2new),
        store.get(node2Idx)
      )).runAsync

      testScheduler.tick(5.seconds)

      val case2Result = case2.futureValue
      case2Result should contain theSameElementsInOrderAs Seq((), node2, (), node2new)

    }
  }
}
