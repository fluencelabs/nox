package fluence.btree.server

import java.nio.ByteBuffer

import fluence.btree.client.MerkleBTreeClient
import fluence.btree.client.MerkleBTreeClient.ClientState
import fluence.btree.common.merkle.MerkleRootCalculator
import fluence.btree.common.{ Hash, Key }
import fluence.btree.server.commands.{ GetCommandImpl, PutCommandImpl }
import fluence.btree.server.core.{ BTreeBinaryStore, NodeOps }
import fluence.codec.kryo.KryoCodecs
import fluence.crypto.cipher.NoOpCrypt
import fluence.crypto.hash.JdkCryptoHasher
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

  private val hasher = JdkCryptoHasher.Sha256
  //    private val hasher = TestCryptoHasher
  private val mRCalc = MerkleRootCalculator(hasher)

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
        val bTree = createBTree()

        val result = for {
          getCb ← client.initGet(key1)
          res ← bTree.get(GetCommandImpl(getCb))
        } yield res shouldBe None

        wait(result)
      }

      "put key1, get key1 from empty tree" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val client = createBTreeClient()
        val bTree = createBTree()
        val counter = Atomic(0L)

        val result = for {
          getCb1 ← client.initGet(key1)
          res1 ← bTree.get(GetCommandImpl(getCb1))
          putCb ← client.initPut(key1, val1.getBytes)
          res2 ← bTree.put(PutCommandImpl(mRCalc, putCb, () ⇒ counter.incrementAndGet()))
          getCb2 ← client.initGet(key1)
          res3 ← bTree.get(GetCommandImpl(getCb2))
        } yield {
          res1 shouldBe None
          res2 shouldBe 1l
          res3 shouldBe Some(1l)
        }

        wait(result)

      }

      "put many value in random order and get theirs" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val minKey = "k0001"
        val midKey = "k0256"
        val maxKey = "k0512"
        val absentKey = "k2048"

        val client = createBTreeClient()
        val bTree = createBTree()
        val counter = Atomic(0L)

        // insert 1024 unique values

        val putRes1 = wait(Task.gather(
          Random.shuffle(1 to 512).map(i ⇒ {
            for {
              putCb ← client.initPut(f"k$i%04d", f"v$i%04d".getBytes)
              res ← bTree.put(PutCommandImpl(mRCalc, putCb, () ⇒ counter.incrementAndGet()))
            } yield res
          })
        ))

        putRes1 should have size 512
        putRes1 should contain allElementsOf (1 to 512)

        // get some values

        val min = wait(for {
          getMinCb ← client.initGet(minKey)
          min ← bTree.get(GetCommandImpl(getMinCb))
        } yield min)
        val mid = wait(for {
          getMidCb ← client.initGet(midKey)
          mid ← bTree.get(GetCommandImpl(getMidCb))
        } yield mid)
        val max = wait(for {
          getMaxCb ← client.initGet(maxKey)
          max ← bTree.get(GetCommandImpl(getMaxCb))
        } yield max)
        val absent = wait(for {
          getAbsentCb ← client.initGet(absentKey)
          absent ← bTree.get(GetCommandImpl(getAbsentCb))
        } yield absent)

        min shouldBe defined
        mid shouldBe defined
        max shouldBe defined
        absent shouldBe None

        // insert 512 new and 512 duplicated values
        val putRes2 = wait(Task.gather(
          Random.shuffle(1 to 1024).map(i ⇒ {
            for {
              putCb ← client.initPut(f"k$i%04d", f"v$i%04d new".getBytes)
              res ← bTree.put(PutCommandImpl(mRCalc, putCb, () ⇒ counter.incrementAndGet()))
            } yield res
          })
        ))

        putRes2 should have size 1024
        putRes2 should contain allElementsOf (1 to 1024)

        // get some values
        val minNew = wait(for {
          getMinCb ← client.initGet(minKey)
          min ← bTree.get(GetCommandImpl(getMinCb))
        } yield min)
        val midNew = wait(for {
          getMidCb ← client.initGet(midKey)
          mid ← bTree.get(GetCommandImpl(getMidCb))
        } yield mid)
        val maxNew = wait(for {
          getMaxCb ← client.initGet(maxKey)
          max ← bTree.get(GetCommandImpl(getMaxCb))
        } yield max)
        val absentNew = wait(for {
          getAbsentCb ← client.initGet(absentKey)
          absent ← bTree.get(GetCommandImpl(getAbsentCb))
        } yield absent)

        minNew shouldBe min
        midNew shouldBe mid
        maxNew shouldBe defined
        absentNew shouldBe None

      }

    }

  }

  /* util methods */

  private def createBTreeClient(clientState: Option[ClientState] = None): MerkleBTreeClient[String] = {
    val keyCrypt = NoOpCrypt.forString
    MerkleBTreeClient(
      clientState,
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
