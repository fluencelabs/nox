package fluence.btree.server.integration

import java.nio.ByteBuffer

import fluence.btree.client.MerkleBTreeClient.ClientState
import fluence.btree.client.{ Key, MerkleBTreeClient, Value }
import fluence.btree.server._
import fluence.btree.server.binary.BTreeBinaryStore
import fluence.crypto.NoOpCrypt
import fluence.hash.TestCryptoHasher
import fluence.node.binary.kryo.KryoCodecs
import fluence.node.storage.InMemoryKVStore
import monix.eval.Task
import monix.execution.ExecutionModel
import monix.execution.schedulers.TestScheduler
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.{ Matchers, WordSpec }

import scala.concurrent.duration._
import scala.math.Ordering

class IntegrationMerkleBTreeSpec extends WordSpec with Matchers with ScalaFutures {

  implicit object BytesOrdering extends Ordering[Array[Byte]] {
    override def compare(x: Array[Byte], y: Array[Byte]): Int = ByteBuffer.wrap(x).compareTo(ByteBuffer.wrap(y))
  }

  private val key1 = "k0001"
  private val val1 = "v0001"
  private val key2 = "k0002"
  private val val2 = "v0002"
  private val key3 = "k0003"
  private val val3 = "v0003"
  private val key4 = "k0004"
  private val val4 = "v0004"
  private val key5 = "k0005"
  private val val5 = "v0005"

  "put and get" should {
    "save and return correct results" when {
      "get from empty tree" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val client = createBTReeClient()

        val result = wait(client.get(key1))
        result shouldBe None
      }

      "put key1, get key1 from empty tree" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val client = createBTReeClient()

        val res1 = wait(client.get(key1))
        val res2 = wait(client.put(key1, val1))
        val res3 = wait(client.get(key1))

        res1 shouldBe None
        res2 shouldBe None
        res3 shouldBe Some(val1)

      }
    }

    // todo add many test cases
  }

  /* util methods */

  //      private val hasher = JdkCryptoHasher.Sha256
  private val hasher = TestCryptoHasher

  private def createBTReeClient(clientState: Option[ClientState] = None): MerkleBTreeClient[String, String] = {
    val keyCrypt = NoOpCrypt.forString
    val valueCrypt = NoOpCrypt.forString
    MerkleBTreeClient(clientState, new LocalBTreeRpc(createBTRee(), hasher), keyCrypt, valueCrypt, hasher)
  }

  private def createBTRee(): MerkleBTree = {

    val codecs = KryoCodecs()
      .add[Key]
      .add[Array[Key]]
      .add[Value]
      .add[Array[Value]]
      .add[NodeId]
      .add[Array[NodeId]]
      .add[Int]
      .add[Node]
      .addCase(classOf[Leaf])
      .addCase(classOf[Branch])
      .build[Task]()

    import codecs._

    val Arity = 4
    val Alpha = 0.25F

    val store = new BTreeBinaryStore[NodeId, Node, Task](InMemoryKVStore())
    new MerkleBTree(MerkleBTreeConfig(arity = Arity, alpha = Alpha), store, NodeOps(hasher))
  }

  private def wait[T](task: Task[T], time: FiniteDuration = 3.second)(implicit TS: TestScheduler): T = {
    val async = task.runAsync
    TS.tick(time)
    async.futureValue
  }

}
