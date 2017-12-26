package fluence.btree.server

import java.nio.ByteBuffer

import fluence.btree.client.MerkleBTreeClient.ClientState
import fluence.btree.client.merkle.MerkleRootCalculator
import fluence.btree.client.protocol.BTreeRpc
import fluence.btree.client.protocol.BTreeRpc.{ GetCallbacks, PutCallbacks }
import fluence.btree.client.{ Key, MerkleBTreeClient, Value }
import fluence.btree.server.binary.BTreeBinaryStore
import fluence.btree.server.commands.{ GetCommandImpl, PutCommandImpl }
import fluence.crypto.NoOpCrypt
import fluence.hash.JdkCryptoHasher
import fluence.codec.kryo.KryoCodecs
import fluence.node.storage.TrieMapKVStore
import monix.eval.Task
import monix.execution.ExecutionModel
import monix.execution.schedulers.TestScheduler
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.{ Matchers, WordSpec }

import scala.collection.concurrent.TrieMap
import scala.concurrent.duration._
import scala.math.Ordering
import scala.util.Random
import scala.util.hashing.MurmurHash3

class IntegrationMerkleBTreeSpec extends WordSpec with Matchers with ScalaFutures {

  // todo add real encryption for keys and values

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

        val client = createBTreeClient()

        val result = wait(client.get(key1))
        result shouldBe None
      }

      "put key1, get key1 from empty tree" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val client = createBTreeClient()

        val res1 = wait(client.get(key1))
        val res2 = wait(client.put(key1, val1))
        val res3 = wait(client.get(key1))

        res1 shouldBe None
        res2 shouldBe None
        res3 shouldBe Some(val1)
      }

      "put many value in random order and get theirs" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val minKey = "k0001"
        val midKey = "k0256"
        val maxKey = "k0512"
        val absentKey = "k2048"

        val client = createBTreeClient()

        // insert 1024 unique values
        val resultMap = wait(Task.gather(
          Random.shuffle(1 to 512)
            .map(i ⇒ client.put(f"k$i%04d", f"v$i%04d")))
        )

        resultMap should have size 512
        resultMap should contain only None

        // get some values
        wait(client.get(minKey)) shouldBe Some("v0001")
        wait(client.get(midKey)) shouldBe Some("v0256")
        wait(client.get(maxKey)) shouldBe Some("v0512")
        wait(client.get(absentKey)) shouldBe None

        // insert 1024 new and 1024 duplicated values
        val result2Map = wait(Task.gather(
          Random.shuffle(1 to 1024)
            .map(i ⇒ client.put(f"k$i%04d", f"v$i%04d new")))
        )

        result2Map should have size 1024
        result2Map.filter(_.isDefined) should have size 512
        result2Map.filter(_.isDefined) should contain allElementsOf (1 to 512).map(i ⇒ Some(f"v$i%04d"))

        // get some values
        wait(client.get(minKey)) shouldBe Some("v0001 new")
        wait(client.get(midKey)) shouldBe Some("v0256 new")
        wait(client.get(maxKey)) shouldBe Some("v0512 new")
        wait(client.get(absentKey)) shouldBe None
      }

    }

  }

  /* util methods */

  private val hasher = JdkCryptoHasher.Sha256
  //  private val hasher = TestCryptoHasher

  private def createBTreeClient(clientState: Option[ClientState] = None): MerkleBTreeClient[String, String] = {
    val keyCrypt = NoOpCrypt.forString
    val valueCrypt = NoOpCrypt.forString
    val bTree = createBTree()
    MerkleBTreeClient(
      clientState,
      new BTreeRpc[Task] {
        override def get(getCallbacks: GetCallbacks[Task]): Task[Unit] =
          bTree.get(new GetCommandImpl[Task](getCallbacks))
        override def put(putCallback: PutCallbacks[Task]): Task[Unit] =
          bTree.put(new PutCommandImpl[Task](MerkleRootCalculator(hasher), putCallback))
      },
      keyCrypt,
      valueCrypt,
      hasher
    )
  }

  private def createBTree(): MerkleBTree = {

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

    val tMap = new TrieMap[Array[Byte], Array[Byte]](MurmurHash3.arrayHashing, Equiv.fromComparator(BytesOrdering))
    val store = new BTreeBinaryStore[NodeId, Node, Task](new TrieMapKVStore[Task, Key, Value](tMap))
    new MerkleBTree(MerkleBTreeConfig(arity = Arity, alpha = Alpha), store, NodeOps(hasher))
  }

  private def wait[T](task: Task[T], time: FiniteDuration = 3.second)(implicit TS: TestScheduler): T = {
    val async = task.runAsync
    TS.tick(time)
    async.futureValue
  }

}
