package fluence.btree.server

import java.nio.ByteBuffer

import fluence.btree.client.MerkleBTreeClient
import fluence.btree.client.MerkleBTreeClient.ClientState
import fluence.btree.common.merkle.MerkleRootCalculator
import fluence.btree.common.{ Hash, Key }
import fluence.btree.protocol.BTreeRpc
import fluence.btree.protocol.BTreeRpc.{ GetCallbacks, PutCallbacks }
import fluence.btree.server.commands.{ GetCommandImpl, PutCommandImpl }
import fluence.btree.server.core.{ BTreeBinaryStore, NodeOps }
import fluence.crypto.hash.JdkCryptoHasher
import fluence.codec.kryo.KryoCodecs
import fluence.crypto.cipher.NoOpCrypt
import fluence.storage.TrieMapKVStore
import monix.eval.Task
import monix.execution.ExecutionModel
import monix.execution.atomic.Atomic
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
        val res2 = wait(client.put(key1, val1.getBytes))
        val res3 = wait(client.get(key1))

        res1 shouldBe None
        res2 shouldBe None
        res3.get shouldBe val1.getBytes
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
            .map(i ⇒ client.put(f"k$i%04d", f"v$i%04d".getBytes)))
        )

        resultMap should have size 512
        resultMap should contain only None

        // get some values
        wait(client.get(minKey)).get shouldBe "v0001".getBytes
        wait(client.get(midKey)).get shouldBe "v0256".getBytes
        wait(client.get(maxKey)).get shouldBe "v0512".getBytes
        wait(client.get(absentKey)) shouldBe None

        // insert 1024 new and 1024 duplicated values
        val result2Map = wait(Task.gather(
          Random.shuffle(1 to 1024)
            .map(i ⇒ client.put(f"k$i%04d", f"v$i%04d new".getBytes)))
        )

        result2Map should have size 1024
        result2Map.filter(_.isDefined) should have size 512
        result2Map.filter(_.isDefined).map(_.get) should contain allElementsOf (1 to 512).map(i ⇒ f"v$i%04d".getBytes)

        // get some values
        wait(client.get(minKey)).get shouldBe "v0001 new".getBytes
        wait(client.get(midKey)).get shouldBe "v0256 new".getBytes
        wait(client.get(maxKey)).get shouldBe "v0512 new".getBytes
        wait(client.get(absentKey)) shouldBe None
      }

    }

  }

  /* util methods */

  private val hasher = JdkCryptoHasher.Sha256
  //  private val hasher = TestCryptoHasher

  private def createBTreeClient(clientState: Option[ClientState] = None): MerkleBTreeClient[String] = {
    val keyCrypt = NoOpCrypt.forString
    val valueCrypt = NoOpCrypt.forString
    val bTree = createBTree()
    val idx = Atomic(0l)
    MerkleBTreeClient(
      clientState,
      new BTreeRpc[Task] {
        override def get(getCallbacks: GetCallbacks[Task]): Task[Unit] =
          bTree.get(new GetCommandImpl[Task](getCallbacks)).map(_ ⇒ ())
        override def put(putCallback: PutCallbacks[Task]): Task[Unit] =
          bTree.put(new PutCommandImpl[Task](MerkleRootCalculator(hasher), putCallback, () ⇒ idx.incrementAndGet())).map(_ ⇒ ())
      },
      keyCrypt,
      hasher
    )
  }

  private def createBTree(): MerkleBTree = {

    val codecs = KryoCodecs()
      .add[Key]
      .add[Array[Key]]
      .add[Hash]
      .add[Array[Hash]]
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
    val store = new BTreeBinaryStore[Task, NodeId, Node](new TrieMapKVStore[Task, Key, Hash](tMap))
    new MerkleBTree(MerkleBTreeConfig(arity = Arity, alpha = Alpha), store, NodeOps(hasher))
  }

  private def wait[T](task: Task[T], time: FiniteDuration = 3.second)(implicit TS: TestScheduler): T = {
    val async = task.runAsync
    TS.tick(time)
    async.futureValue
  }

}
