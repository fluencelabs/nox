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

package fluence.btree.server

import java.nio.ByteBuffer

import fluence.btree.common._
import fluence.btree.common.merkle.MerkleRootCalculator
import fluence.btree.core.{ ClientPutDetails, Hash, Key }
import fluence.btree.protocol.BTreeRpc.{ PutCallbacks, SearchCallback }
import fluence.btree.server.commands.{ PutCommandImpl, SearchCommandImpl }
import fluence.btree.server.core.{ BTreeBinaryStore, NodeOps, SearchCommand }
import fluence.codec.kryo.KryoCodecs
import fluence.storage.TrieMapKVStore
import monix.eval.Task
import monix.execution.ExecutionModel
import monix.execution.atomic.Atomic
import monix.execution.schedulers.TestScheduler
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.{ BeforeAndAfterAll, Matchers, WordSpec }
import scodec.bits.ByteVector
import slogging.{ LogLevel, LoggerConfig, PrintLoggerFactory }

import scala.collection.Searching.Found
import scala.collection.concurrent.TrieMap
import scala.concurrent.duration._
import scala.math.Ordering
import scala.util.Random
import scala.util.hashing.MurmurHash3

class MerkleBTreeSpec extends WordSpec with Matchers with ScalaFutures with BeforeAndAfterAll {

  implicit class Str2Key(str: String) {
    def toKey: Key = Key(str.getBytes)
  }

  implicit class Str2Hash(str: String) {
    def toHash: Hash = Hash(str.getBytes)
  }

  implicit object BytesOrdering extends Ordering[Array[Byte]] {
    override def compare(x: Array[Byte], y: Array[Byte]): Int = ByteBuffer.wrap(x).compareTo(ByteBuffer.wrap(y))
  }

  implicit object KeyOrdering extends Ordering[Key] {
    override def compare(x: Key, y: Key): Int = BytesOrdering.compare(x.bytes, y.bytes)
  }

  private val hasher = TestHasher()

  private val Arity = 4
  private val Alpha = 0.25F
  private val Config = MerkleBTreeConfig(arity = Arity, alpha = Alpha)
  private val MinSize = (Arity * Alpha).toInt
  private val MaxSize = Arity

  private val key1: Key = "k0001".toKey
  private val value1: Hash = "v0001".toHash
  private val valRef1: Long = 1l
  private val key2: Key = "k0002".toKey
  private val value2: Hash = "v0002".toHash
  private val valRef2: Long = 2l
  private val key3: Key = "k0003".toKey
  private val value3: Hash = "v0003".toHash
  private val valRef3: Long = 3l
  private val key4: Key = "k0004".toKey
  private val value4: Hash = "v0004".toHash
  private val valRef4: Long = 4l
  private val key5: Key = "k0005".toKey
  private val value5: Hash = "v0005".toHash
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
    .add[Option[NodeId]]
    .add[None.type]

    .addCase(classOf[Leaf])
    .addCase(classOf[Branch])
    .build[Task]()

  import codecs._

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
        root.rightSibling shouldBe None
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
        root.rightSibling shouldBe None
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
        root.rightSibling shouldBe None
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
        checkTree(Array(key2), Array(2, 1), root)

        val rootChildren = root.childsReferences.map(childId ⇒ wait(store.get(childId)))
        rootChildren should have size 2
        rootChildren.foreach(child ⇒ checkNodeValidity(child))
        rootChildren.head.asInstanceOf[Leaf].rightSibling shouldBe Some(1L)
        rootChildren.tail.head.asInstanceOf[Leaf].rightSibling shouldBe None
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
        // only rightmost leaf should be without right sibling
        rootChildren.map { case l: Leaf ⇒ l.rightSibling }.count(_.isEmpty) shouldBe 1
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
        // only rightmost leaf should be without right sibling
        rootChildren.map { case l: Leaf ⇒ l.rightSibling }.count(_.isEmpty) shouldBe 1
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
          override def verifyChanges(serverMerkleRoot: Hash, wasSplitting: Boolean): Task[Unit] = Task(())
          override def changesStored(): Task[Unit] = Task(())
          override def nextChildIndex(keys: Array[Key], childsChecksums: Array[Hash]): Task[Int] = ???
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
          override def verifyChanges(serverMerkleRoot: Hash, wasSplitting: Boolean): Task[Unit] = Task(())
          override def changesStored(): Task[Unit] = Task(())
          override def nextChildIndex(keys: Array[Key], childsChecksums: Array[Hash]): Task[Int] = ???
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
        wait(Task.sequence(putCmd(1 to 1) map { cmd ⇒ tree.put(cmd) }))

        val result = wait(tree.get(failedSearchCmd(key1, SendLeafStage)).failed)
        result.getMessage shouldBe "Client unavailable"
      }

      "something wrong with searching next child" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val tree: MerkleBTree = createTree()

        wait(Task.sequence(putCmd(1 to 5) map { cmd ⇒ tree.put(cmd) }))
        val result = wait(tree.get(failedSearchCmd(key1, NextChildIndexStage)).failed)
        result.getMessage shouldBe "Client unavailable"

      }
    }

    "return result" when {
      "value not found" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val tree: MerkleBTree = createTree()

        wait(tree.get(searchCmd(key1, { result ⇒ result shouldBe None }))) shouldBe None
      }

      "value found in root-leaf with one value" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val tree: MerkleBTree = createTree()

        wait(Task.sequence(putCmd(1 to 1) map { cmd ⇒ tree.put(cmd) }))
        wait(tree.get(searchCmd(key1, { result ⇒ result.get.bytes shouldBe value1.bytes }))) shouldBe Some(valRef1)
      }

      "value found in filled root-leaf" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val tree: MerkleBTree = createTree()

        wait(Task.sequence(putCmd(1 to 4) map { cmd ⇒ tree.put(cmd) }))
        wait(tree.get(searchCmd(key3, { result ⇒ result.get.bytes shouldBe value3.bytes }))) shouldBe Some(valRef3)
      }

      "value found in huge tree" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val tree: MerkleBTree = createTree()

        wait(Task.sequence(putCmd(Random.shuffle(1 to 512)) map { cmd ⇒ tree.put(cmd) }))

        val minKey = "k0001".toKey
        val midKey = "k0256".toKey
        val maxKey = "k0512".toKey
        val absentKey = "k2048".toKey

        wait(tree.get(searchCmd(minKey, { result ⇒ result.get.bytes shouldBe "v0001".getBytes }))) shouldBe defined
        wait(tree.get(searchCmd(midKey, { result ⇒ result.get.bytes shouldBe "v0256".getBytes }))) shouldBe defined
        wait(tree.get(searchCmd(maxKey, { result ⇒ result.get.bytes shouldBe "v0512".getBytes }))) shouldBe defined
        wait(tree.get(searchCmd(absentKey, { result ⇒ result shouldBe None }))) shouldBe None
      }
    }
  }

  "range" should {
    "show error from client or network" when {
      "something wrong with sending leaf to client" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val tree: MerkleBTree = createTree()

        wait(Task.sequence(putCmd(1 to 1) map { cmd ⇒ tree.put(cmd) }))
        val result = wait(tree.range(failedSearchCmd(key1, SendLeafStage)).toListL.failed)
        result.getMessage shouldBe "Client unavailable"
      }

      "something wrong with searching next child" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val tree: MerkleBTree = createTree()

        wait(Task.sequence(putCmd(1 to 5) map { cmd ⇒ tree.put(cmd) }))
        val result = wait(tree.range(failedSearchCmd(key1, NextChildIndexStage)).toListL.failed)
        result.getMessage shouldBe "Client unavailable"

      }
    }

    "return stream of pairs" when {
      "value not found" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val tree: MerkleBTree = createTree()

        val result = wait(tree.range(searchCmd(key1, { result ⇒ result shouldBe None })).toListL)
        result shouldBe empty
      }

      "value found in root-leaf with one value" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val tree: MerkleBTree = createTree()

        wait(Task.sequence(putCmd(1 to 1) map { cmd ⇒ tree.put(cmd) }))
        val result = wait(tree.range(searchCmd(key1, { result ⇒ result.get.bytes shouldBe value1.bytes })).toListL)
        verifyRangeResults(result, List(key1 → valRef1))
      }

      "value found in filled root-leaf" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val tree: MerkleBTree = createTree()

        wait(Task.sequence(putCmd(1 to 4) map { cmd ⇒ tree.put(cmd) }))
        val result = wait(tree.range(searchCmd(key2, { result ⇒ result.get.bytes shouldBe value2.bytes })).toListL)
        verifyRangeResults(result, List(key2 → valRef2, key3 → valRef3, key4 → valRef4))
      }

      "value found in huge tree" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val tree: MerkleBTree = createTree()

        val seq: Seq[Int] = 0 to 256 by 2
        wait(Task.sequence(putCmd(Random.shuffle(seq)) map { cmd ⇒ tree.put(cmd) }))

        val minKey = "k0000".toKey
        val midKey = "k0128".toKey
        val maxKey = "k0256".toKey
        val absentKey = "k9999".toKey

        // case when start key is less than all keys in tree
        val notFoundStartKey = wait(tree.range(searchCmd("abc".toKey, { result ⇒ result shouldBe None })).toListL)
        notFoundStartKey.size shouldBe 129
        notFoundStartKey.head._1.toByteVector shouldBe minKey.toByteVector
        notFoundStartKey.last._1.toByteVector shouldBe maxKey.toByteVector

        val oneElem = wait(tree.range(searchCmd(midKey, { result ⇒ result.get.bytes shouldBe "v0128".getBytes })).take(1).toListL)
        oneElem.size shouldBe 1
        oneElem.head._1.toByteVector shouldBe midKey.toByteVector

        val fromStartTenPairs = wait(tree.range(searchCmd(minKey, { result ⇒ result.get.bytes shouldBe "v0000".getBytes })).take(10).toListL)
        fromStartTenPairs.size shouldBe 10
        fromStartTenPairs.head._1.toByteVector shouldBe minKey.toByteVector
        fromStartTenPairs.last._1.toByteVector shouldBe "k0018".toKey.toByteVector

        val fromMidToEnd = wait(tree.range(searchCmd(midKey, { result ⇒ result.get.bytes shouldBe "v0128".getBytes })).toListL)
        fromMidToEnd.size shouldBe 65
        fromMidToEnd.head._1.toByteVector shouldBe midKey.toByteVector
        fromMidToEnd.last._1.toByteVector shouldBe maxKey.toByteVector

        val fromLastTenPairs = wait(tree.range(searchCmd(maxKey, { result ⇒ result.get.bytes shouldBe "v0256".getBytes })).take(10).toListL)
        fromLastTenPairs.size shouldBe 1
        fromLastTenPairs.head._1.toByteVector shouldBe maxKey.toByteVector

        val fromSkippedKeyOneElem = wait(tree.range(searchCmd("k0001".toKey, { result ⇒ result shouldBe None })).take(1).toListL)
        fromSkippedKeyOneElem.size shouldBe 1
        fromSkippedKeyOneElem.head._1.toByteVector shouldBe "k0002".toKey.toByteVector

        val fromSkippedKeyTenElem = wait(tree.range(searchCmd("k0001".toKey, { result ⇒ result shouldBe None })).take(10).toListL)
        fromSkippedKeyTenElem.size shouldBe 10
        fromSkippedKeyTenElem.head._1.toByteVector shouldBe "k0002".toKey.toByteVector
        fromSkippedKeyTenElem.last._1.toByteVector shouldBe "k0020".toKey.toByteVector

        val notOverlap = wait(tree.range(searchCmd(absentKey, { result ⇒ result shouldBe None })).toListL)
        notOverlap shouldBe empty

      }

    }

  }

  "put, get and range" should {
    "save and return correct results" when {
      "put key1, get key1" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val tree: MerkleBTree = createTree()

        val putRes = wait(Task.sequence(putCmd(1 to 1) map { cmd ⇒ tree.put(cmd) }))
        val getRes = wait(tree.get(searchCmd(key1, { result ⇒ result.get.bytes shouldBe value1.bytes })))
        val rangeRes = wait(tree.range(searchCmd(key1, { result ⇒ result.get.bytes shouldBe value1.bytes })).toListL)

        putRes.head shouldBe getRes.get
        rangeRes.size shouldBe 1
        rangeRes.head._1.toByteVector shouldBe key1.toByteVector
      }
    }

    "put many value in random order and get theirs" in {
      implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
      val tree: MerkleBTree = createTree()

      wait(Task.sequence(putCmd(Random.shuffle(1 to 1024)) map { cmd ⇒ tree.put(cmd) }))

      tree.getDepth should be >= 5

      val minKey = "k0001".toKey
      val midKey = "k0512".toKey
      val maxKey = "k1024".toKey
      val absentKey = "k2048".toKey

      wait(tree.get(searchCmd(minKey, { result ⇒ result.get.bytes shouldBe "v0001".getBytes }))) shouldBe defined
      wait(tree.get(searchCmd(midKey, { result ⇒ result.get.bytes shouldBe "v0512".getBytes }))) shouldBe defined
      wait(tree.get(searchCmd(maxKey, { result ⇒ result.get.bytes shouldBe "v1024".getBytes }))) shouldBe defined
      wait(tree.get(searchCmd(absentKey, { result ⇒ result shouldBe None }))) shouldBe None

      val fetchAll = wait(tree.range(searchCmd(minKey, { result ⇒ result.get.bytes shouldBe value1.bytes })).toListL)
      fetchAll.size shouldBe 1024

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

      val minKey = "k0001".toKey
      val midKey = "k0512".toKey
      val maxKey = "k1024".toKey
      val absentKey = "k2048".toKey

      wait(tree.get(searchCmd(minKey, { result ⇒ result.get.bytes shouldBe "v0001".getBytes }))) shouldBe defined
      wait(tree.get(searchCmd(midKey, { result ⇒ result.get.bytes shouldBe "v0512".getBytes }))) shouldBe defined
      wait(tree.get(searchCmd(maxKey, { result ⇒ result.get.bytes shouldBe "v1024".getBytes }))) shouldBe defined
      wait(tree.get(searchCmd(absentKey, { result ⇒ result shouldBe None }))) shouldBe None

      val fetchAll = wait(tree.range(searchCmd(minKey, { result ⇒ result.get.bytes shouldBe value1.bytes })).toListL)
      fetchAll.size shouldBe 1024
    }

  }

  /* util methods */

  private def createTreeStore = {
    val blobIdCounter = Atomic(0L)
    val tMap = new TrieMap[Array[Byte], Array[Byte]](MurmurHash3.arrayHashing, Equiv.fromComparator(BytesOrdering))
    new BTreeBinaryStore[Task, NodeId, Node](new TrieMapKVStore[Task, Array[Byte], Array[Byte]](tMap), () ⇒ blobIdCounter.incrementAndGet())
  }

  private def createTree(store: BTreeBinaryStore[Task, NodeId, Node] = createTreeStore): MerkleBTree =
    new MerkleBTree(MerkleBTreeConfig(arity = Arity), store, nodeOp)

  private def wait[T](task: Task[T], time: FiniteDuration = 3.second)(implicit TS: TestScheduler): T = {
    val async = task.runAsync
    TS.tick(time)
    async.futureValue
  }

  private def checkLeaf(expKeys: Array[Key], expValRef: Array[ValueRef], expValHash: Array[Hash], node: Leaf): Unit = {
    node.keys.map(_.bytes) should contain theSameElementsInOrderAs expKeys.map(_.bytes)
    node.valuesReferences should contain theSameElementsInOrderAs expValRef
    node.valuesChecksums.asStr should contain theSameElementsInOrderAs expValHash.asStr
    node.size shouldBe expKeys.length
    node.checksum.bytes should not be empty
  }

  private def checkTree(expKeys: Array[Key], expChildren: Array[NodeId], tree: Branch): Unit = {
    tree.keys.map(_.bytes) should contain theSameElementsInOrderAs expKeys.map(_.bytes)
    tree.childsReferences should contain theSameElementsInOrderAs expChildren
    tree.size shouldBe expKeys.length
    tree.checksum.bytes should not be empty
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
        leaf.checksum.bytes should not be empty
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
    node.checksum.bytes should not be empty
  }

  /** Creates Seq of PutCommand for specified Range of key indexes. */
  private def putCmd(seq: Seq[Int]): Seq[PutCommandImpl[Task]] = {
    val idx = Atomic(0L)
    seq map { i ⇒
      new PutCommandImpl[Task](
        mRootCalculator, new PutCallbacks[Task] {
        import scala.collection.Searching._
        override def putDetails(keys: Array[Key], values: Array[Hash]): Task[ClientPutDetails] =
          Task(ClientPutDetails(f"k$i%04d".toKey, f"v$i%04d".toHash, keys.search(f"k$i%04d".toKey)))
        override def verifyChanges(serverMerkleRoot: Hash, wasSplitting: Boolean): Task[Unit] =
          Task(())
        override def changesStored(): Task[Unit] =
          Task(())
        override def nextChildIndex(keys: Array[Key], childsChecksums: Array[Hash]): Task[Int] =
          Task(keys.search(f"k$i%04d".toKey).insertionPoint)
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
            Task(ClientPutDetails(f"k$i%04d".toKey, f"v$i%04d".toHash, keys.search(f"k$i%04d".toKey)))
        }
        override def verifyChanges(serverMerkleRoot: Hash, wasSplitting: Boolean): Task[Unit] = {
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
        override def nextChildIndex(keys: Array[Key], childsChecksums: Array[Hash]): Task[Int] = {
          if (stageOfFail == NextChildIndexStage)
            Task.raiseError(new Exception(errMsg))
          else
            Task(keys.search(f"k$i%04d".toKey).insertionPoint)
        }
      }, () ⇒ idx.incrementAndGet())
    }
  }

  /** Search value for specified key and return callback for searched result */
  private def searchCmd(
    key: Key,
    resultFn: Option[Hash] ⇒ Unit = { _ ⇒ () }
  ): SearchCommand[Task, Key, ValueRef, NodeId] = {
    SearchCommandImpl[Task](new SearchCallback[Task] {
      import scala.collection.Searching._
      override def submitLeaf(keys: Array[Key], values: Array[Hash]): Task[SearchResult] = {
        val result = keys.search(key)
        resultFn(result match {
          case Found(i) ⇒
            Some(values(i))
          case _ ⇒
            None
        })

        Task(result)
      }
      override def nextChildIndex(keys: Array[Key], childsChecksums: Array[Hash]): Task[Int] =
        Task(keys.search(key).insertionPoint)

    })
  }

  private sealed trait GetStage
  private case object SendLeafStage extends GetStage

  /** Search value for specified key and raise exception for specified BTreeServerResponse type */
  private def failedSearchCmd[T](
    key: Key,
    stageOfFail: GetStage,
    errMsg: String = "Client unavailable"
  ): SearchCommand[Task, Key, ValueRef, NodeId] = {
    SearchCommandImpl[Task](new SearchCallback[Task] {
      import scala.collection.Searching._
      override def submitLeaf(keys: Array[Key], values: Array[Hash]): Task[SearchResult] = {
        if (stageOfFail == SendLeafStage)
          Task.raiseError(new Exception(errMsg))
        else
          Task(InsertionPoint(0))
      }
      override def nextChildIndex(keys: Array[Key], childsChecksums: Array[Hash]): Task[Int] = {
        if (stageOfFail == NextChildIndexStage)
          Task.raiseError(new Exception(errMsg))
        else
          Task(keys.search(key).insertionPoint)
      }
    })
  }

  private def verifyRangeResults(rangeRes: List[(Key, ValueRef)], expected: List[(Key, ValueRef)]): Unit =
    keys2BV(rangeRes) should contain theSameElementsInOrderAs keys2BV(expected)

  private def keys2BV(seq: List[(Key, ValueRef)]): List[(ByteVector, ValueRef)] =
    seq.map { case (key, ref) ⇒ key.toByteVector -> ref }

  private implicit class Hashes2Strings(hashArr: Array[Hash]) {
    def asStr: Array[String] = hashArr.map(h ⇒ new String(h.bytes))
  }

  override protected def beforeAll(): Unit = {
    LoggerConfig.factory = PrintLoggerFactory
    LoggerConfig.level = LogLevel.OFF
    super.beforeAll()
  }

  override protected def afterAll(): Unit = {
    super.afterAll()
    LoggerConfig.level = LogLevel.OFF
  }

}
