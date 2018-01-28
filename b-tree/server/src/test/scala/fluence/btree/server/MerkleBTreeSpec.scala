package fluence.btree.server

import java.nio.ByteBuffer

import fluence.btree.common.merkle.MerkleRootCalculator
import fluence.btree.common._
import fluence.btree.protocol.BTreeRpc.{ GetCallbacks, PutCallbacks }
import fluence.btree.server.commands.{ GetCommandImpl, PutCommandImpl }
import fluence.btree.server.core.{ BTreeBinaryStore, BTreePutDetails, NodeOps }
import fluence.crypto.hash.TestCryptoHasher
import fluence.codec.kryo.KryoCodecs
import fluence.storage.TrieMapKVStore
import monix.eval.Task
import monix.execution.ExecutionModel
import monix.execution.atomic.Atomic
import monix.execution.schedulers.TestScheduler
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.{ Matchers, WordSpec }

import scala.collection.Searching.Found
import scala.collection.concurrent.TrieMap
import scala.concurrent.duration._
import scala.math.Ordering
import scala.util.Random
import scala.util.hashing.MurmurHash3

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
  private val value1: Hash = "v0001".getBytes()
  private val valRef1: Long = 1l
  private val key2: Key = "k0002".getBytes()
  private val value2: Hash = "v0002".getBytes()
  private val valRef2: Long = 2l
  private val key3: Key = "k0003".getBytes()
  private val value3: Hash = "v0003".getBytes()
  private val valRef3: Long = 3l
  private val key4: Key = "k0004".getBytes()
  private val value4: Hash = "v0004".getBytes()
  private val valRef4: Long = 4l
  private val key5: Key = "k0005".getBytes()
  private val value5: Hash = "v0005".getBytes()
  private val valRef5: Long = 5l

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

  //    val hasher = JdkCryptoHash.Sha256
  val hasher = TestCryptoHasher
  val nodeOp = NodeOps(hasher)
  private val mRootCalculator = MerkleRootCalculator(hasher)

  "put" should {
    "show error from client or network" when {
      "something wrong with sending leaf to client" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val tree: MerkleBTree = createTree()

        val result = wait(Task.sequence(failedPutCmd(1 to 1, PutDetailsStage) map { cmd ⇒ tree.put(cmd) }).failed)
        result.getMessage shouldBe "Client unavailable"
      }

      "something wrong with verifying changes by client" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val tree: MerkleBTree = createTree()

        val result = wait(Task.sequence(failedPutCmd(1 to 1, VerifyChangesStage) map { cmd ⇒ tree.put(cmd) }).failed)

        result.getMessage shouldBe "Client unavailable"

      }
    }

    "correct insert new value" when {
      "tree is empty" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val tree: MerkleBTree = createTree()

        val putRes = wait(Task.sequence(putCmd(1 to 1) map { cmd ⇒ tree.put(cmd) }))

        putRes shouldBe Seq(1l)
        // check tree state
        tree.getDepth shouldBe 1
        val root = wait(tree.getRoot).asInstanceOf[Leaf]
        checkLeaf(Array(key1), Array(valRef1), Array(value1), root)
      }

      "tree contains 1 element, insertion key is less than key in tree" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val tree: MerkleBTree = createTree()

        val putRes = wait(Task.sequence(putCmd(2 to (1, -1)) map { cmd ⇒ tree.put(cmd) }))

        putRes shouldBe Seq(1l, 2l)
        // check tree state
        tree.getDepth shouldBe 1
        val root = wait(tree.getRoot).asInstanceOf[Leaf]
        checkLeaf(Array(key1, key2), Array(valRef2, valRef1), Array(value1, value2), root)
        checkNodeValidity(root)
      }

      "tree contains 1 element, insertion key is more than key in tree" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val tree: MerkleBTree = createTree()

        val putRes = wait(Task.sequence(putCmd(1 to 2) map { cmd ⇒ tree.put(cmd) }))

        putRes shouldBe Seq(1l, 2l)
        // check tree state
        tree.getDepth shouldBe 1
        val root = wait(tree.getRoot).asInstanceOf[Leaf]
        checkLeaf(Array(key1, key2), Array(valRef1, valRef2), Array(value1, value2), root)
        checkNodeValidity(root)
      }

      "tree has filled root-leaf" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val store = createTreeStore
        val tree: MerkleBTree = createTree(store)

        val putRes = wait(Task.sequence(putCmd(1 to 5) map { cmd ⇒ tree.put(cmd) }))

        putRes shouldBe Seq(1l, 2l, 3l, 4l, 5l)
        tree.getDepth shouldBe 2
        val root = wait(tree.getRoot).asInstanceOf[Branch]
        checkNodeValidity(root)
        checkTree(Array(key2), Array(1, 2), root)

        val rootChildren: Array[Node] = root.childsReferences.map(childId ⇒ wait(store.get(childId)))
        rootChildren should have size 2
        rootChildren.foreach(child ⇒ checkNodeValidity(child))

      }

      "many put operation with ascending keys (only leaf is spiting)" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val store = createTreeStore
        val tree: MerkleBTree = createTree(store)

        val putRes = wait(Task.sequence(putCmd(1 to 11) map { cmd ⇒ tree.put(cmd) }))

        putRes shouldBe (1 to 11).map(_.toLong)
        val root = wait(tree.getRoot).asInstanceOf[Branch]
        checkNodeValidity(root)

        tree.getDepth shouldBe 2
        val rootChildren: Array[Node] = root.childsReferences.map(childId ⇒ wait(store.get(childId)))
        rootChildren.foldLeft(0)((acc, node) ⇒ acc + node.size) shouldBe 11
        rootChildren.foreach(child ⇒ checkNodeValidity(child))

      }

      "many put operation with descending keys (only leaf is spiting)" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val store = createTreeStore
        val tree: MerkleBTree = createTree(store)

        val putRes = wait(Task.sequence(putCmd(11 to (1, -1)) map { cmd ⇒ tree.put(cmd) }))

        putRes shouldBe (1 to 11).map(_.toLong)
        val root = wait(tree.getRoot).asInstanceOf[Branch]
        checkNodeValidity(root)

        tree.getDepth shouldBe 2
        val rootChildren: Array[Node] = root.childsReferences.map(childId ⇒ wait(store.get(childId)))
        rootChildren.foldLeft(0)((acc, node) ⇒ acc + node.size) shouldBe 11
        rootChildren.foreach(child ⇒ checkNodeValidity(child))
      }

      "many put operation with ascending keys (leafs and trees are splitting)" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val store = createTreeStore
        val tree: MerkleBTree = createTree(store)

        val putRes = wait(Task.sequence(putCmd(1 to 32) map { cmd ⇒ tree.put(cmd) }))

        putRes shouldBe (1 to 32).map(_.toLong)
        val root = wait(tree.getRoot).asInstanceOf[Branch]
        checkNodeValidity(root)

        tree.getDepth shouldBe 4
        val rootChildren: Array[Node] = root.childsReferences.map(childId ⇒ wait(store.get(childId)))
        rootChildren.foreach(child ⇒ checkNodeValidity(child))
      }

      "many put operation with descending keys (leafs and trees are splitting)" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val store = createTreeStore
        val tree: MerkleBTree = createTree(store)

        val putRes = wait(Task.sequence(putCmd(32 to (1, -1)) map { cmd ⇒ tree.put(cmd) }))

        putRes shouldBe (1 to 32).map(_.toLong)
        val root = wait(tree.getRoot).asInstanceOf[Branch]
        checkNodeValidity(root)

        tree.getDepth shouldBe 3
        val rootChildren: Array[Node] = root.childsReferences.map(childId ⇒ wait(store.get(childId)))
        rootChildren.foreach(child ⇒ checkNodeValidity(child))
      }

      "many put operation with random keys (leafs and trees are splitting)" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val store = createTreeStore
        val tree: MerkleBTree = createTree(store)

        val putRes = wait(Task.sequence(putCmd(Random.shuffle(1 to 32)) map { cmd ⇒ tree.put(cmd) }))

        putRes should contain allElementsOf (1 to 32).map(_.toLong)
        val root = wait(tree.getRoot).asInstanceOf[Branch]
        checkNodeValidity(root)

        tree.getDepth should be >= 3
        val rootChildren: Array[Node] = root.childsReferences.map(childId ⇒ wait(store.get(childId)))
        rootChildren.foreach(child ⇒ checkNodeValidity(child))
      }
    }

    "correct update value" when {
      "tree has 1 element" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val tree: MerkleBTree = createTree()

        val putRes1 = wait(Task.sequence(putCmd(1 to 1) map { cmd ⇒ tree.put(cmd) }))
        val putRes2 = wait(tree.put(new PutCommandImpl[Task](mRootCalculator, new PutCallbacks[Task] {
          override def putDetails(
            keys: Array[Key],
            values: Array[Hash]
          ): Task[ClientPutDetails] = Task(ClientPutDetails(key1, value2, Found(0)))
          override def verifyChanges(serverMerkleRoot: Bytes, wasSplitting: Boolean): Task[Unit] = Task(())
          override def changesStored(): Task[Unit] = Task(())
          override def nextChildIndex(keys: Array[Key], childsChecksums: Array[Bytes]): Task[Int] = ???
        }, () ⇒ 2l)))

        putRes1 shouldBe Seq(1l)
        putRes2 shouldBe 1l
        // check tree state
        tree.getDepth shouldBe 1
        val root = wait(tree.getRoot).asInstanceOf[Leaf]
        checkLeaf(Array(key1), Array(valRef1), Array(value2), root) // valRef1 is old ref - it's correct behaviour
      }

      "tree has filled root-leaf" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val tree: MerkleBTree = createTree()

        val putRes1 = wait(Task.sequence(putCmd(1 to 4) map { cmd ⇒ tree.put(cmd) }))
        val putRes2 = wait(tree.put(new PutCommandImpl[Task](mRootCalculator, new PutCallbacks[Task] {
          val idx = Atomic(0L)

          override def putDetails(
            keys: Array[Key],
            values: Array[Hash]
          ): Task[ClientPutDetails] = Task(ClientPutDetails(key2, value5, Found(1)))
          override def verifyChanges(serverMerkleRoot: Bytes, wasSplitting: Boolean): Task[Unit] = Task(())
          override def changesStored(): Task[Unit] = Task(())
          override def nextChildIndex(keys: Array[Key], childsChecksums: Array[Bytes]): Task[Int] = ???
        }, () ⇒ 5l)))

        putRes1 shouldBe Seq(1l, 2l, 3l, 4l)
        putRes2 shouldBe 2l
        tree.getDepth shouldBe 1
        val root = wait(tree.getRoot).asInstanceOf[Leaf] // valRef2 is old ref - it's correct behaviour
        checkLeaf(Array(key1, key2, key3, key4), Array(valRef1, valRef2, valRef3, valRef4), Array(value1, value5, value3, value4), root)
      }
    }
  }

  "get" should {
    "show error from client or network" when {
      "something wrong with sending leaf to client" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val tree: MerkleBTree = createTree()

        val result = wait(tree.get(failedGetCmd(key1, SendLeafStage)).failed)
        result.getMessage shouldBe "Client unavailable"
      }

      "something wrong with searching next child" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val tree: MerkleBTree = createTree()

        wait(Task.sequence(putCmd(1 to 5) map { cmd ⇒ tree.put(cmd) }))
        val result = wait(tree.get(failedGetCmd(key1, NextChildIndexStage)).failed)
        result.getMessage shouldBe "Client unavailable"

      }
    }

    "return empty result" when {
      "value not found" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val tree: MerkleBTree = createTree()

        wait(tree.get(getCmd(key1, { result ⇒ result shouldBe None }))) shouldBe None
      }

      "value found in root-leaf with one value" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val tree: MerkleBTree = createTree()

        wait(Task.sequence(putCmd(1 to 1) map { cmd ⇒ tree.put(cmd) }))
        wait(tree.get(getCmd(key1, { result ⇒ result.get shouldBe value1 }))) shouldBe Some(valRef1)
      }

      "value found in filled root-leaf" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val tree: MerkleBTree = createTree()

        wait(Task.sequence(putCmd(1 to 4) map { cmd ⇒ tree.put(cmd) }))
        wait(tree.get(getCmd(key3, { result ⇒ result.get shouldBe value3 }))) shouldBe Some(valRef3)
      }

      "value found in huge tree" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val tree: MerkleBTree = createTree()

        wait(Task.sequence(putCmd(Random.shuffle(1 to 512)) map { cmd ⇒ tree.put(cmd) }))

        val minKey = "k0001".getBytes
        val midKey = "k0256".getBytes
        val maxKey = "k0512".getBytes
        val absentKey = "k2048".getBytes

        wait(tree.get(getCmd(minKey, { result ⇒ result.get shouldBe "v0001".getBytes }))) shouldBe defined
        wait(tree.get(getCmd(midKey, { result ⇒ result.get shouldBe "v0256".getBytes }))) shouldBe defined
        wait(tree.get(getCmd(maxKey, { result ⇒ result.get shouldBe "v0512".getBytes }))) shouldBe defined
        wait(tree.get(getCmd(absentKey, { result ⇒ result shouldBe None }))) shouldBe None
      }

    }

  }

  "put and get" should {
    "save and return correct results" when {
      "put key1, get key1" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val tree: MerkleBTree = createTree()

        val putRes = wait(Task.sequence(putCmd(1 to 1) map { cmd ⇒ tree.put(cmd) }))
        val getRes = wait(tree.get(getCmd(key1, { result ⇒ result.get shouldBe value1 })))
        putRes.head shouldBe getRes.get
      }
    }

    "put many value in random order and get theirs" in {
      implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
      val tree: MerkleBTree = createTree()

      wait(Task.sequence(putCmd(Random.shuffle(1 to 1024)) map { cmd ⇒ tree.put(cmd) }))

      tree.getDepth should be >= 5

      val minKey = "k0001".getBytes
      val midKey = "k0512".getBytes
      val maxKey = "k1024".getBytes
      val absentKey = "k2048".getBytes

      wait(tree.get(getCmd(minKey, { result ⇒ result.get shouldBe "v0001".getBytes }))) shouldBe defined
      wait(tree.get(getCmd(midKey, { result ⇒ result.get shouldBe "v0512".getBytes }))) shouldBe defined
      wait(tree.get(getCmd(maxKey, { result ⇒ result.get shouldBe "v1024".getBytes }))) shouldBe defined
      wait(tree.get(getCmd(absentKey, { result ⇒ result shouldBe None }))) shouldBe None

    }

    "put twice many value in random order and get theirs" in {
      implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
      val tree: MerkleBTree = createTree()

      // put 1024 elements
      val putRes1 = wait(Task.sequence(putCmd(Random.shuffle(1 to 1024)) map { cmd ⇒ tree.put(cmd) }))

      // put 1024 elements again
      val putRes2 = wait(Task.sequence(putCmd(Random.shuffle(1 to 1024)) map { cmd ⇒ tree.put(cmd) }))

      putRes1 should contain allElementsOf putRes2

      tree.getDepth should be >= 5

      val minKey = "k0001".getBytes
      val midKey = "k0512".getBytes
      val maxKey = "k1024".getBytes
      val absentKey = "k2048".getBytes

      wait(tree.get(getCmd(minKey, { result ⇒ result.get shouldBe "v0001".getBytes }))) shouldBe defined
      wait(tree.get(getCmd(midKey, { result ⇒ result.get shouldBe "v0512".getBytes }))) shouldBe defined
      wait(tree.get(getCmd(maxKey, { result ⇒ result.get shouldBe "v1024".getBytes }))) shouldBe defined
      wait(tree.get(getCmd(absentKey, { result ⇒ result shouldBe None }))) shouldBe None
    }

  }

  /* util methods */

  private def createTreeStore = {
    val tMap = new TrieMap[Key, Hash](MurmurHash3.arrayHashing, Equiv.fromComparator(BytesOrdering))
    new BTreeBinaryStore[Task, NodeId, Node](new TrieMapKVStore[Task, Key, Hash](tMap))
  }

  private def createTree(store: BTreeBinaryStore[Task, NodeId, Node] = createTreeStore): MerkleBTree =
    new MerkleBTree(MerkleBTreeConfig(arity = Arity), store, nodeOp)

  private def wait[T](task: Task[T], time: FiniteDuration = 3.second)(implicit TS: TestScheduler): T = {
    val async = task.runAsync
    TS.tick(time)
    async.futureValue
  }

  private def checkLeaf(expKeys: Array[Key], expValRef: Array[ValueRef], expValHash: Array[Hash], node: Leaf): Unit = {
    node.keys should contain theSameElementsInOrderAs expKeys
    node.valuesReferences should contain theSameElementsInOrderAs expValRef
    node.valuesChecksums should contain theSameElementsInOrderAs expValHash
    node.size shouldBe expKeys.length
    node.checksum should not be empty
  }

  private def checkTree(expKeys: Array[Key], expChildren: Array[NodeId], tree: Branch): Unit = {
    tree.keys should contain theSameElementsInOrderAs expKeys
    tree.childsReferences should contain theSameElementsInOrderAs expChildren
    tree.size shouldBe expKeys.length
    tree.checksum should not be empty
  }

  private def checkNodeValidity(node: Node, min: Int = MinSize, max: Int = MaxSize): Unit = {
    node match {
      case tree: Branch @unchecked ⇒
        checkNodeSize(tree, min, max)
        checkOrderOfKeys(tree.keys)
        tree.childsReferences.length should be >= tree.size
      case leaf: Node @unchecked ⇒
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

  /** Creates Seq of PutCommand for specified Range of key indexes. */
  private def putCmd(seq: Seq[Int]): Seq[PutCommandImpl[Task]] = {
    val idx = Atomic(0L)
    seq map { i ⇒
      new PutCommandImpl[Task](
        mRootCalculator, new PutCallbacks[Task] {
        import scala.collection.Searching._
        override def putDetails(keys: Array[Key], values: Array[Hash]): Task[ClientPutDetails] =
          Task(ClientPutDetails(f"k$i%04d".getBytes(), f"v$i%04d".getBytes(), keys.search(f"k$i%04d".getBytes())))
        override def verifyChanges(serverMerkleRoot: Bytes, wasSplitting: Boolean): Task[Unit] =
          Task(())
        override def changesStored(): Task[Unit] =
          Task(())
        override def nextChildIndex(keys: Array[Key], childsChecksums: Array[Bytes]): Task[Int] =
          Task(keys.search(f"k$i%04d".getBytes()).insertionPoint)
      }, () ⇒ idx.incrementAndGet())
    }
  }

  private sealed trait PutStage
  private case object NextChildIndexStage extends PutStage with GetStage
  private case object PutDetailsStage extends PutStage
  private case object VerifyChangesStage extends PutStage
  private case object ChangesStoredStage extends PutStage

  /**
   * Creates Seq of PutCommand for specified Range of key indexes and raise exception
   * for specified BTreeServerResponse type.
   */
  private def failedPutCmd[T](
    seq: Seq[Int],
    stageOfFail: PutStage,
    errMsg: String = "Client unavailable"
  ): Seq[PutCommandImpl[Task]] = {
    val idx = Atomic(0L)
    seq map { i ⇒
      new PutCommandImpl[Task](
        mRootCalculator, new PutCallbacks[Task] {
        import scala.collection.Searching._
        override def putDetails(keys: Array[Key], values: Array[Hash]): Task[ClientPutDetails] = {
          if (stageOfFail == PutDetailsStage)
            Task.raiseError(new Exception(errMsg))
          else
            Task(ClientPutDetails(f"k$i%04d".getBytes(), f"v$i%04d".getBytes(), keys.search(f"k$i%04d".getBytes())))
        }
        override def verifyChanges(serverMerkleRoot: Bytes, wasSplitting: Boolean): Task[Unit] = {
          if (stageOfFail == VerifyChangesStage)
            Task.raiseError(new Exception(errMsg))
          else
            Task(())
        }
        override def changesStored(): Task[Unit] = {
          if (stageOfFail == ChangesStoredStage)
            Task.raiseError(new Exception(errMsg))
          else
            Task(())
        }
        override def nextChildIndex(keys: Array[Key], childsChecksums: Array[Bytes]): Task[Int] = {
          if (stageOfFail == NextChildIndexStage)
            Task.raiseError(new Exception(errMsg))
          else
            Task(keys.search(f"k$i%04d".getBytes()).insertionPoint)
        }
      }, () ⇒ idx.incrementAndGet())
    }
  }

  /** Search value for specified key and return callback for searched result */
  private def getCmd(key: Key, resultFn: Option[Hash] ⇒ Unit = { _ ⇒ () }): Get = {
    new GetCommandImpl[Task](new GetCallbacks[Task] {
      import scala.collection.Searching._
      override def submitLeaf(keys: Array[Key], values: Array[Hash]): Task[Option[Int]] = {
        val result = keys.search(key) match {
          case Found(i) ⇒
            Some(i)
          case _ ⇒
            None
        }

        resultFn(result.map(values(_)))
        Task(result)
      }
      override def nextChildIndex(keys: Array[Key], childsChecksums: Array[Bytes]): Task[Int] =
        Task(keys.search(key).insertionPoint)

    })
  }
  private sealed trait GetStage
  private case object SendLeafStage extends GetStage

  /** Search value for specified key and raise exception for specified BTreeServerResponse type */
  private def failedGetCmd[T](
    key: Key,
    stageOfFail: GetStage,
    errMsg: String = "Client unavailable"
  ): Get = {
    new GetCommandImpl[Task](new GetCallbacks[Task] {
      import scala.collection.Searching._
      override def submitLeaf(keys: Array[Key], values: Array[Hash]): Task[Option[Int]] = {
        if (stageOfFail == SendLeafStage)
          Task.raiseError(new Exception(errMsg))
        else
          Task(None)
      }
      override def nextChildIndex(keys: Array[Key], childsChecksums: Array[Bytes]): Task[Int] = {
        if (stageOfFail == NextChildIndexStage)
          Task.raiseError(new Exception(errMsg))
        else
          Task(keys.search(key).insertionPoint)
      }
    })
  }

}
