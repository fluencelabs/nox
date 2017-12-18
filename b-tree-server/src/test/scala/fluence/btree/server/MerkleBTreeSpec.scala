package fluence.btree.server

import java.nio.ByteBuffer

import fluence.btree.client.{ BTreeMerkleProofInspector, Key, Value, WriteResults }
import fluence.btree.server.binary.BTreeBinaryStore
import fluence.node.binary.kryo.KryoCodecs
import fluence.btree.server.core._
import fluence.hash.TestCryptoHasher
import fluence.node.storage.InMemoryKVStore
import monix.eval.Task
import monix.execution.ExecutionModel
import monix.execution.schedulers.TestScheduler
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.{ Matchers, WordSpec }

import scala.concurrent.duration._
import scala.math.Ordering
import scala.util.Random

class MerkleBTreeSpec extends WordSpec with Matchers with ScalaFutures {

  implicit object BytesOrdering extends Ordering[Array[Byte]] {
    override def compare(x: Array[Byte], y: Array[Byte]): Int = ByteBuffer.wrap(x).compareTo(ByteBuffer.wrap(y))
  }

  private val Arity = 4
  private val Alpha = 0.25F
  private val Config = MerkleBTreeConfig(arity = Arity, alpha = Alpha)
  private val MinSize = (Arity * Alpha).toInt
  private val MaxSize = Arity

  private val key1: Key = "k0001".getBytes()
  private val value1: Value = "v0001".getBytes()
  private val key2: Key = "k0002".getBytes()
  private val value2: Value = "v0002".getBytes()
  private val key3: Key = "k0003".getBytes()
  private val value3: Value = "v0003".getBytes()
  private val key4: Key = "k0004".getBytes()
  private val value4: Value = "v0004".getBytes()
  private val key5: Key = "k0005".getBytes()
  private val value5: Value = "v0005".getBytes()

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

  //    val hasher = JdkCryptoHash.Sha256
  val hasher = TestCryptoHasher
  val nodeOp = NodeOps(hasher)
  val inspector = BTreeMerkleProofInspector(hasher)

  "put" should {
    "correct insert new value" when {
      "tree is empty" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val store = new BTreeBinaryStore[NodeId, Node, Task](InMemoryKVStore())
        val tree = new MerkleBTree(Config, store, LocalRouter[Key, Task], nodeOp)
        val clientMerkleRoot = Array.emptyByteArray

        val result = wait(tree.put(key1, value1))

        // check returned result
        result.key shouldBe key1
        result.value shouldBe value1
        inspector.verifyPut(result, clientMerkleRoot) shouldBe true

        // check tree state
        tree.getDepth shouldBe 1
        val root = wait(tree.getRoot).asInstanceOf[Leaf]
        checkLeaf(Array(key1), Array(value1), root)

      }

      "tree contains 1 element, insertion key is less than key in tree" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val store = new BTreeBinaryStore[NodeId, Node, Task](InMemoryKVStore())
        val router = LocalRouter[Key, Task]
        val tree = new MerkleBTree(Config, store, router, nodeOp)

        // put first value
        val firstPutResult = wait(tree.put(key2, value2))
        val clientMerkleRoot = inspector.calcNewMerkleRoot(firstPutResult)

        // put second value
        val result = wait(tree.put(key1, value1))

        // check returned result
        result.key shouldBe key1
        result.value shouldBe value1
        inspector.verifyPut(result, clientMerkleRoot) shouldBe true

        // check tree state
        tree.getDepth shouldBe 1
        val root = wait(tree.getRoot).asInstanceOf[Leaf]
        checkLeaf(Array(key1, key2), Array(value1, value2), root)
        checkNodeValidity(root)
      }

      "tree contains 1 element, insertion key is more than key in tree" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val store = new BTreeBinaryStore[NodeId, Node, Task](InMemoryKVStore())
        val tree = new MerkleBTree(Config, store, LocalRouter[Key, Task], nodeOp)

        val firstPutResult = wait(tree.put(key1, value1))
        val secondPutResult = wait(tree.put(key2, value2))

        // check returned result
        secondPutResult.key shouldBe key2
        secondPutResult.value shouldBe value2
        checkAllPutProofs(Seq(firstPutResult, secondPutResult))

        // check tree state
        tree.getDepth shouldBe 1

        val root = wait(tree.getRoot).asInstanceOf[Leaf]
        checkLeaf(Array(key1, key2), Array(value1, value2), root)
        checkNodeValidity(root)
      }

      "tree has filled root-leaf" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val store = new BTreeBinaryStore[NodeId, Node, Task](InMemoryKVStore())
        val tree = new MerkleBTree(MerkleBTreeConfig(arity = 4), store, LocalRouter[Key, Task], nodeOp)

        val putTasks = (1 to 5) map { i ⇒ tree.put(f"k$i%04d".getBytes(), f"v$i%04d".getBytes()) }
        val results = wait(Task.sequence(putTasks))

        tree.getDepth shouldBe 2
        checkAllPutProofs(results)

        val root = wait(tree.getRoot).asInstanceOf[Branch]
        checkNodeValidity(root)
        checkTree(Array(key2), Array(1, 2), root)

        val rootChildren: Array[Node] = root.children.map(childId ⇒ wait(store.get(childId)))
        rootChildren should have size 2
        rootChildren.foreach(child ⇒ checkNodeValidity(child))

      }

      "many put operation with ascending keys (only leaf is spiting)" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val store = new BTreeBinaryStore[NodeId, Node, Task](InMemoryKVStore())
        val tree = new MerkleBTree(MerkleBTreeConfig(arity = 4), store, LocalRouter[Key, Task], nodeOp)

        val putTasks = (1 to 11) map { i ⇒ tree.put(f"k$i%04d".getBytes(), f"v$i%04d".getBytes()) }

        val results = wait(Task.sequence(putTasks))
        checkAllPutProofs(results)

        val root = wait(tree.getRoot).asInstanceOf[Branch]
        checkNodeValidity(root)

        tree.getDepth shouldBe 2
        val rootChildren: Array[Node] = root.children.map(childId ⇒ wait(store.get(childId)))
        rootChildren.foldLeft(0)((acc, node) ⇒ acc + node.size) shouldBe 11
        rootChildren.foreach(child ⇒ checkNodeValidity(child))

      }

      "many put operation with descending keys (only leaf is spiting)" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val store = new BTreeBinaryStore[NodeId, Node, Task](InMemoryKVStore())
        val tree = new MerkleBTree(MerkleBTreeConfig(arity = 4), store, LocalRouter[Key, Task], nodeOp)

        val putTasks = (11 to (1, -1)) map { i ⇒ tree.put(f"k$i%04d".getBytes(), f"v$i%04d".getBytes()) }

        val results = wait(Task.sequence(putTasks))
        checkAllPutProofs(results)

        val root = wait(tree.getRoot).asInstanceOf[Branch]
        checkNodeValidity(root)

        tree.getDepth shouldBe 2
        val rootChildren: Array[Node] = root.children.map(childId ⇒ wait(store.get(childId)))
        rootChildren.foldLeft(0)((acc, node) ⇒ acc + node.size) shouldBe 11
        rootChildren.foreach(child ⇒ checkNodeValidity(child))
      }

      "many put operation with ascending keys (leafs and trees are splitting)" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val store = new BTreeBinaryStore[NodeId, Node, Task](InMemoryKVStore())
        val tree = new MerkleBTree(MerkleBTreeConfig(arity = 4), store, LocalRouter[Key, Task], nodeOp)

        val putTasks = (1 to 32) map { i ⇒ tree.put(f"k$i%04d".getBytes(), f"v$i%04d".getBytes()) }

        val results = wait(Task.sequence(putTasks))
        checkAllPutProofs(results)

        val root = wait(tree.getRoot).asInstanceOf[Branch]
        checkNodeValidity(root)

        tree.getDepth shouldBe 4
        val rootChildren: Array[Node] = root.children.map(childId ⇒ wait(store.get(childId)))
        rootChildren.foreach(child ⇒ checkNodeValidity(child))
      }

      "many put operation with descending keys (leafs and trees are splitting)" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val store = new BTreeBinaryStore[NodeId, Node, Task](InMemoryKVStore())
        val tree = new MerkleBTree(MerkleBTreeConfig(arity = 4), store, LocalRouter[Key, Task], nodeOp)

        val putTasks = (32 to (1, -1)) map { i ⇒ tree.put(f"k$i%04d".getBytes(), f"v$i%04d".getBytes()) }

        val results = wait(Task.sequence(putTasks))
        checkAllPutProofs(results)

        val root = wait(tree.getRoot).asInstanceOf[Branch]
        checkNodeValidity(root)

        tree.getDepth shouldBe 3
        val rootChildren: Array[Node] = root.children.map(childId ⇒ wait(store.get(childId)))
        rootChildren.foreach(child ⇒ checkNodeValidity(child))
      }

      "many put operation with random keys (leafs and trees are splitting)" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val store = new BTreeBinaryStore[NodeId, Node, Task](InMemoryKVStore())
        val tree = new MerkleBTree(MerkleBTreeConfig(arity = 4), store, LocalRouter[Key, Task], nodeOp)

        val seq = Random.shuffle(1 to 32)
        val putTasks = seq map { i ⇒ tree.put(f"k$i%04d".getBytes(), f"v$i%04d".getBytes()) }

        val results = wait(Task.sequence(putTasks))
        checkAllPutProofs(results)

        val root = wait(tree.getRoot).asInstanceOf[Branch]
        checkNodeValidity(root)

        tree.getDepth should be >= 3
        val rootChildren: Array[Node] = root.children.map(childId ⇒ wait(store.get(childId)))
        rootChildren.foreach(child ⇒ checkNodeValidity(child))
      }
    }

    "correct update value" when {
      "tree has 1 element" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val store = new BTreeBinaryStore[NodeId, Node, Task](InMemoryKVStore())
        val tree = new MerkleBTree(Config, store, LocalRouter[Key, Task], nodeOp)

        // put first value
        val firstResult = wait(tree.put(key1, value1))
        val firstMerkleRoot = inspector.calcNewMerkleRoot(firstResult)
        // put second value
        val secondResult = wait(tree.put(key1, value2))

        // check returned result
        secondResult.key shouldBe key1
        secondResult.value shouldBe value2
        inspector.verifyPut(secondResult, firstMerkleRoot) shouldBe true

        // check tree state
        tree.getDepth shouldBe 1
        val root = wait(tree.getRoot).asInstanceOf[Leaf]
        checkLeaf(Array(key1), Array(value2), root)
      }

      "tree has filled root-leaf" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val store = new BTreeBinaryStore[NodeId, Node, Task](InMemoryKVStore())
        val tree = new MerkleBTree(MerkleBTreeConfig(arity = 4), store, LocalRouter[Key, Task], nodeOp)

        val putTasks = (1 to 4) map { i ⇒ tree.put(f"k$i%04d".getBytes(), f"v$i%04d".getBytes()) }
        val results = wait(Task.sequence(putTasks))

        checkAllPutProofs(results) // check proofs before update
        val updateResult = wait(tree.put(key2, value5))
        checkAllPutProofs(results :+ updateResult) // check proofs after update

        tree.getDepth shouldBe 1
        val root = wait(tree.getRoot).asInstanceOf[Leaf]
        checkLeaf(Array(key1, key2, key3, key4), Array(value1, value5, value3, value4), root)
      }

    }
  }

  "get" should {
    "return empty result" when {
      "value not found" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val store = new BTreeBinaryStore[NodeId, Node, Task](InMemoryKVStore())
        val tree = new MerkleBTree(Config, store, LocalRouter[Key, Task], nodeOp)

        val result = wait(tree.get(key1))

        result.key shouldBe key1
        result.value shouldBe None
        result.proof.path shouldBe Array()
      }

      "value found in root-leaf with one value" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val store = new BTreeBinaryStore[NodeId, Node, Task](InMemoryKVStore())
        val tree = new MerkleBTree(Config, store, LocalRouter[Key, Task], nodeOp)

        wait(tree.put(key1, value1))

        val result = wait(tree.get(key1))

        result.key shouldBe key1
        result.value.get shouldBe value1
        result.proof.path should have size 1
        val merkleRoot = wait(tree.getMerkleRoot)
        inspector.verifyGet(result, merkleRoot) shouldBe true

      }

      "value found in filled root-leaf" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val store = new BTreeBinaryStore[NodeId, Node, Task](InMemoryKVStore())
        val tree = new MerkleBTree(Config, store, LocalRouter[Key, Task], nodeOp)

        wait(tree.put(key1, value1))
        wait(tree.put(key2, value2))
        wait(tree.put(key3, value3))
        wait(tree.put(key4, value4))

        val result = wait(tree.get(key3))

        result.key shouldBe key3
        result.value.get shouldBe value3
        result.proof.path should have size 1
        val merkleRoot = wait(tree.getMerkleRoot)
        inspector.verifyGet(result, merkleRoot) shouldBe true
      }

      "value found in huge tree" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val store = new BTreeBinaryStore[NodeId, Node, Task](InMemoryKVStore())
        val tree = new MerkleBTree(Config, store, LocalRouter[Key, Task], nodeOp)

        val seq = Random.shuffle(1 to 512)
        val putTasks = seq map { i ⇒ tree.put(f"k$i%04d".getBytes(), f"v$i%04d".getBytes()) }

        val putResults = wait(Task.gather(putTasks))

        val minKey = "k0001".getBytes
        val midKey = "k0256".getBytes
        val maxKey = "k0512".getBytes
        val absentKey = "k2048".getBytes

        val minResult = wait(tree.get(minKey))
        val midResult = wait(tree.get(midKey))
        val maxResult = wait(tree.get(maxKey))
        val absentResult = wait(tree.get(absentKey))

        val merkleRoot = wait(tree.getMerkleRoot)

        minResult.key shouldBe minKey
        minResult.value.get shouldBe "v0001".getBytes
        inspector.verifyGet(minResult, merkleRoot) shouldBe true

        midResult.key shouldBe midKey
        midResult.value.get shouldBe "v0256".getBytes
        inspector.verifyGet(midResult, merkleRoot) shouldBe true

        maxResult.key shouldBe maxKey
        maxResult.value.get shouldBe "v0512".getBytes
        inspector.verifyGet(maxResult, merkleRoot) shouldBe true

        absentResult.key shouldBe absentKey
        absentResult.value shouldBe None
        inspector.verifyGet(absentResult, merkleRoot) shouldBe true
      }

    }

  }

  "put and get" should {
    "save and return correct results" when {
      "put key1, get key1" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val store = new BTreeBinaryStore[NodeId, Node, Task](InMemoryKVStore())
        val tree = new MerkleBTree(Config, store, LocalRouter[Key, Task], nodeOp)

        wait(tree.put(key1, value1))
        val result = wait(tree.get(key1))

        result.key shouldBe key1
        result.value.get shouldBe value1
        val merkleRoot = wait(tree.getMerkleRoot)
        inspector.verifyGet(result, merkleRoot)
      }
    }

    "put many value in random order and get theirs" in {
      implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
      val store = new BTreeBinaryStore[NodeId, Node, Task](InMemoryKVStore())
      val tree = new MerkleBTree(MerkleBTreeConfig(arity = Arity), store, LocalRouter[Key, Task], nodeOp)

      val seq = Random.shuffle(1 to 1024)
      val putTasks = seq map { i ⇒ tree.put(f"k$i%04d".getBytes(), f"v$i%04d".getBytes()) }

      val results = wait(Task.sequence(putTasks))
      checkAllPutProofs(results)
      tree.getDepth should be >= 5

      val minKey = "k0001".getBytes
      val midKey = "k0512".getBytes
      val maxKey = "k1024".getBytes
      val absentKey = "k2048".getBytes

      val minResult = wait(tree.get(minKey))
      val midResult = wait(tree.get(midKey))
      val maxResult = wait(tree.get(maxKey))
      val absentResult = wait(tree.get(absentKey))
      val merkleRoot = wait(tree.getMerkleRoot)

      minResult.key shouldBe minKey
      minResult.value.get shouldBe "v0001".getBytes
      inspector.verifyGet(minResult, merkleRoot) shouldBe true

      midResult.key shouldBe midKey
      midResult.value.get shouldBe "v0512".getBytes
      inspector.verifyGet(midResult, merkleRoot) shouldBe true

      maxResult.key shouldBe maxKey
      maxResult.value.get shouldBe "v1024".getBytes
      inspector.verifyGet(maxResult, merkleRoot) shouldBe true

      absentResult.key shouldBe absentKey
      absentResult.value shouldBe None
      inspector.verifyGet(absentResult, merkleRoot) shouldBe true

    }

    "put twice many value in random order and get theirs" in {
      implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
      val store = new BTreeBinaryStore[NodeId, Node, Task](InMemoryKVStore())
      val tree = new MerkleBTree(MerkleBTreeConfig(arity = Arity), store, LocalRouter[Key, Task], nodeOp)

      val seq = Random.shuffle(1 to 1024)
      val putTasks = seq map { i ⇒ tree.put(f"k$i%04d".getBytes(), f"v$i%04d".getBytes()) }

      // put 1024 elements
      val results1 = wait(Task.sequence(putTasks))
      checkAllPutProofs(results1)

      // put 1024 elements again
      val results2 = wait(Task.sequence(putTasks))
      checkAllPutProofs(results2)

      tree.getDepth should be >= 5

      val minKey = "k0001".getBytes
      val midKey = "k0512".getBytes
      val maxKey = "k1024".getBytes
      val absentKey = "k2048".getBytes

      val minResult = wait(tree.get(minKey))
      val midResult = wait(tree.get(midKey))
      val maxResult = wait(tree.get(maxKey))
      val absentResult = wait(tree.get(absentKey))
      val merkleRoot = wait(tree.getMerkleRoot)

      minResult.key shouldBe minKey
      minResult.value.get shouldBe "v0001".getBytes
      inspector.verifyGet(minResult, merkleRoot) shouldBe true

      midResult.key shouldBe midKey
      midResult.value.get shouldBe "v0512".getBytes
      inspector.verifyGet(midResult, merkleRoot) shouldBe true

      maxResult.key shouldBe maxKey
      maxResult.value.get shouldBe "v1024".getBytes
      inspector.verifyGet(maxResult, merkleRoot) shouldBe true

      absentResult.key shouldBe absentKey
      absentResult.value shouldBe None
      inspector.verifyGet(absentResult, merkleRoot) shouldBe true

    }

  }

  /* util methods */

  private def wait[T](task: Task[T], time: FiniteDuration = 3.second)(implicit TS: TestScheduler): T = {
    val async = task.runAsync
    TS.tick(time)
    async.futureValue
  }

  private def checkLeaf(expKeys: Array[Key], expValues: Array[Value], node: Leaf): Unit = {
    node.keys should contain theSameElementsInOrderAs expKeys
    node.values should contain theSameElementsInOrderAs expValues
    node.size shouldBe expKeys.length
    node.checksum should not be empty
  }

  private def checkTree(expKeys: Array[Key], expChildren: Array[NodeId], tree: Branch): Unit = {
    tree.keys should contain theSameElementsInOrderAs expKeys
    tree.children should contain theSameElementsInOrderAs expChildren
    tree.size shouldBe expKeys.length
    tree.checksum should not be empty
  }

  private def checkNodeValidity(node: Node, min: Int = MinSize, max: Int = MaxSize): Unit = {
    node match {
      case tree: Branch ⇒
        checkNodeSize(tree, min, max)
        checkOrderOfKeys(tree.keys)
        tree.children.length should be >= tree.size
      case leaf: Node ⇒
        checkNodeSize(leaf, min, max)
        checkOrderOfKeys(leaf.keys)
        leaf.checksum should not be empty
    }
  }

  private def checkOrderOfKeys(keys: Array[Key]): Unit = {
    keys should have size keys.toSet.size // shouldn't be duplicates
    keys should contain theSameElementsInOrderAs keys.sorted // should be ascending order
  }

  private def checkNodeSize(node: Node, min: Int = MinSize, max: Int = MaxSize): Unit = {
    node.size shouldBe node.keys.length
    node.size should be >= min
    node.size should be <= max
    node.checksum should not be empty
  }

  private def createTree(config: MerkleBTreeConfig = Config) = {
    val store = new BTreeBinaryStore[NodeId, Node, Task](InMemoryKVStore())
    val router = LocalRouter[Key, Task]
    new MerkleBTree(config, store, router, nodeOp)
  }

  private def checkAllPutProofs(results: Seq[WriteResults]): Unit = {
    results.sliding(2).foreach {
      case Seq(prev, next) ⇒
        val prevRoot = inspector.calcNewMerkleRoot(prev)
        if (inspector.verifyPut(next, prevRoot)) {
          succeed
        } else {
          fail(s"verify for hashes failed: " +
            s"prev=${prev.newStateProof} " +
            s"next=${next.oldStateProof} " +
            s"with keys ${new String(prev.key)} ${new String(next.key)}")
        }
    }
  }

}
