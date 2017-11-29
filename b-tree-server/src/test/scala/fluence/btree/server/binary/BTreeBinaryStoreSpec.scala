package fluence.btree.server.binary

import fluence.btree.server.binary.kryo.KryoCodecs
import fluence.node.storage.InMemoryKVStore
import monix.eval.Task
import monix.execution.ExecutionModel
import monix.execution.schedulers.TestScheduler
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.{ Matchers, WordSpec }

import scala.concurrent.duration._

class BTreeBinaryStoreSpec extends WordSpec with Matchers with ScalaFutures {

  "BTreeBinaryStore" should {
    val codecs =
      KryoCodecs()
        .build[Task]()
    import codecs._

    "performs all operations correctly" in {

      implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

      val store = new BTreeBinaryStore[Long, String, Task](InMemoryKVStore())

      val node1 = "node1"
      val node1Idx = 2L
      val node2 = "node2"
      val node2new = "node2"
      val node2Idx = 3L

      // check read absent node

      val case0 = store.get(node1Idx).runAsync

      testScheduler.tick(5.seconds)

      val case0Result = case0.eitherValue
      case0Result.map {
        case left: Left[IllegalArgumentException, _] ⇒ succeed
        case _                                       ⇒ fail()
      }

      // check write and read

      val case1 = Task.sequence(Seq(
        store.put(node1Idx, node1),
        store.get(node1Idx)
      )).runAsync

      testScheduler.tick(5.seconds)

      val case1Result = case1.futureValue
      case1Result should contain theSameElementsInOrderAs Seq((), node1)

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
