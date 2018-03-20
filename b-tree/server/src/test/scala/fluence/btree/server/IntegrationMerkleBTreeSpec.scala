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

import fluence.btree.client.MerkleBTreeClient
import fluence.btree.client.MerkleBTreeClient.ClientState
import fluence.btree.common.merkle.MerkleRootCalculator
import fluence.btree.core.{Hash, Key}
import fluence.btree.server.commands.{PutCommandImpl, SearchCommandImpl}
import fluence.btree.server.core.{BTreeBinaryStore, NodeOps}
import fluence.codec.kryo.KryoCodecs
import fluence.crypto.cipher.NoOpCrypt
import fluence.storage.TrieMapKVStore
import monix.eval.Task
import monix.execution.ExecutionModel
import monix.execution.atomic.Atomic
import monix.execution.schedulers.TestScheduler
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.time.{Milliseconds, Seconds, Span}
import org.scalatest.{BeforeAndAfterAll, Matchers, WordSpec}
import slogging.{LogLevel, LoggerConfig, PrintLoggerFactory}

import scala.collection.concurrent.TrieMap
import scala.concurrent.duration._
import scala.math.Ordering
import scala.util.Random
import scala.util.hashing.MurmurHash3

class IntegrationMerkleBTreeSpec extends WordSpec with Matchers with ScalaFutures with BeforeAndAfterAll {
  override implicit def patienceConfig: PatienceConfig = PatienceConfig(Span(1, Seconds), Span(250, Milliseconds))

  implicit class Str2Key(str: String) {
    def toKey: Key = Key(str.getBytes)
  }

  implicit class Str2Hash(str: String) {
    def toHash: Hash = Hash(str.getBytes)
  }

  implicit object BytesOrdering extends Ordering[Array[Byte]] {
    override def compare(x: Array[Byte], y: Array[Byte]): Int = ByteBuffer.wrap(x).compareTo(ByteBuffer.wrap(y))
  }

  private val blobIdCounter = Atomic(0L)

  private val hasher = TestHasher()
  private val mRCalc = MerkleRootCalculator(hasher)

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

  "put, get and range" should {
    "save and return correct results" when {
      "get from empty tree" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val client = createBTreeClient()
        val bTree = createBTree()

        val getResult = for {
          cb ← client.initGet(key1)
          res ← bTree.get(SearchCommandImpl(cb))
          _ ← cb.recoverState()
        } yield res shouldBe None
        wait(getResult)

        val rangeResult = for {
          cb ← client.initRange(key1)
          res ← bTree.range(SearchCommandImpl(cb)).headOptionL
          _ ← cb.recoverState()
        } yield res shouldBe None
        wait(rangeResult)
      }

      "put key1, get key1 from empty tree" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val client = createBTreeClient()
        val bTree = createBTree()
        val counter = Atomic(0L)

        val getRes1 = wait(for {
          cb ← client.initGet(key1)
          res ← bTree.get(SearchCommandImpl(cb))
          _ ← cb.recoverState()
        } yield res)

        val rangeRes1 = wait(for {
          cb ← client.initRange(key1)
          res ← bTree.range(SearchCommandImpl(cb)).doOnTerminate(_ ⇒ cb.recoverState()).toListL
          _ ← cb.recoverState()
        } yield res)

        val putRes1 = wait(for {
          cb ← client.initPut(key1, val1.toHash)
          res ← bTree.put(PutCommandImpl(mRCalc, cb, () ⇒ counter.incrementAndGet()))
        } yield res)

        val getRes2 = wait(for {
          cb ← client.initGet(key1)
          res ← bTree.get(SearchCommandImpl(cb))
          _ ← cb.recoverState()
        } yield res)

        val rangeRes2 = wait(for {
          cb ← client.initRange(key1)
          res ← bTree.range(SearchCommandImpl(cb)).toListL
          _ ← cb.recoverState()
        } yield res)

        getRes1 shouldBe None
        rangeRes1 shouldBe empty
        putRes1 shouldBe 1L
        getRes2 shouldBe Some(1L)
        rangeRes2.head._1.toByteVector shouldBe key1.toKey.toByteVector

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

        val putRes1 = wait(
          Task.gather(
            Random
              .shuffle(1 to 512)
              .map(i ⇒ {
                for {
                  cb ← client.initPut(f"k$i%04d", f"v$i%04d".toHash)
                  res ← bTree.put(PutCommandImpl(mRCalc, cb, () ⇒ counter.incrementAndGet()))
                } yield res
              })
          ))

        putRes1 should have size 512
        putRes1 should contain allElementsOf (1 to 512)

        // get some values

        val min = wait(for {
          cb ← client.initGet(minKey)
          res ← bTree.get(SearchCommandImpl(cb))
          _ ← cb.recoverState()
        } yield res)
        val mid = wait(for {
          cb ← client.initGet(midKey)
          res ← bTree.get(SearchCommandImpl(cb))
          _ ← cb.recoverState()
        } yield res)
        val max = wait(for {
          cb ← client.initGet(maxKey)
          max ← bTree.get(SearchCommandImpl(cb))
          _ ← cb.recoverState()
        } yield max)
        val absent = wait(for {
          cb ← client.initGet(absentKey)
          res ← bTree.get(SearchCommandImpl(cb))
          _ ← cb.recoverState()
        } yield res)

        min shouldBe defined
        mid shouldBe defined
        max shouldBe defined
        absent shouldBe None

        // get range some values

        val fromMinToMax = wait(for {
          cb ← client.initGet(minKey)
          res ← bTree.range(SearchCommandImpl(cb)).toListL
          _ ← cb.recoverState()
        } yield res)
        val fromMidTenElems = wait(for {
          cb ← client.initGet(midKey)
          res ← bTree.range(SearchCommandImpl(cb)).take(10).toListL
          _ ← cb.recoverState()
        } yield res)
        val fromMaxTenElems = wait(for {
          cb ← client.initGet(maxKey)
          res ← bTree.range(SearchCommandImpl(cb)).take(10).toListL
          _ ← cb.recoverState()
        } yield res)
        val fromLessThanMinTenElems = wait(for {
          cb ← client.initGet("abc")
          res ← bTree.range(SearchCommandImpl(cb)).take(10).toListL
          _ ← cb.recoverState()
        } yield res)

        fromMinToMax.size shouldBe 512
        fromMinToMax.head._1.toByteVector shouldBe minKey.toKey.toByteVector
        fromMinToMax.last._1.toByteVector shouldBe maxKey.toKey.toByteVector
        fromMidTenElems.size shouldBe 10
        fromMidTenElems.head._1.toByteVector shouldBe midKey.toKey.toByteVector
        fromMaxTenElems.size shouldBe 1
        fromMaxTenElems.head._1.toByteVector shouldBe maxKey.toKey.toByteVector
        fromLessThanMinTenElems.size shouldBe 10
        fromLessThanMinTenElems.head._1.toByteVector shouldBe minKey.toKey.toByteVector

        // insert 512 new and 512 duplicated values

        val putRes2 = wait(
          Task.gather(
            Random
              .shuffle(1 to 1024)
              .map(i ⇒ {
                for {
                  putCb ← client.initPut(f"k$i%04d", f"v$i%04d new".toHash)
                  res ← bTree.put(PutCommandImpl(mRCalc, putCb, () ⇒ counter.incrementAndGet()))
                } yield res
              })
          ))

        putRes2 should have size 1024
        putRes2 should contain allElementsOf (1 to 1024)

        // get some values

        val minNew = wait(for {
          cb ← client.initGet(minKey)
          res ← bTree.get(SearchCommandImpl(cb))
          _ ← cb.recoverState()
        } yield res)
        val midNew = wait(for {
          cb ← client.initGet(midKey)
          res ← bTree.get(SearchCommandImpl(cb))
          _ ← cb.recoverState()
        } yield res)
        val maxNew = wait(for {
          cb ← client.initGet(maxKey)
          res ← bTree.get(SearchCommandImpl(cb))
          _ ← cb.recoverState()
        } yield res)
        val absentNew = wait(for {
          cb ← client.initGet(absentKey)
          res ← bTree.get(SearchCommandImpl(cb))
          _ ← cb.recoverState()
        } yield res)

        minNew shouldBe min
        midNew shouldBe mid
        maxNew shouldBe defined
        absentNew shouldBe None

        val newMidKey = "k0512"
        val newMaxKey = "k1024"

        // get range some values

        val fromMinToMaxNew = wait(for {
          cb ← client.initGet(minKey)
          res ← bTree.range(SearchCommandImpl(cb)).toListL
          _ ← cb.recoverState()
        } yield res)
        val fromMidTenElemsNew = wait(for {
          cb ← client.initGet(newMidKey)
          res ← bTree.range(SearchCommandImpl(cb)).take(10).toListL
          _ ← cb.recoverState()
        } yield res)
        val fromMaxTenElemsNew = wait(for {
          cb ← client.initGet(newMaxKey)
          res ← bTree.range(SearchCommandImpl(cb)).take(10).toListL
          _ ← cb.recoverState()
        } yield res)
        val fromLessThanMinTenElemsNew = wait(for {
          cb ← client.initGet("abc")
          res ← bTree.range(SearchCommandImpl(cb)).take(10).toListL
          _ ← cb.recoverState()
        } yield res)

        fromMinToMaxNew.size shouldBe 1024
        fromMinToMaxNew.head._1.toByteVector shouldBe minKey.toKey.toByteVector
        fromMinToMaxNew.last._1.toByteVector shouldBe newMaxKey.toKey.toByteVector
        fromMidTenElemsNew.size shouldBe 10
        fromMidTenElemsNew.head._1.toByteVector shouldBe newMidKey.toKey.toByteVector
        fromMaxTenElemsNew.size shouldBe 1
        fromMaxTenElemsNew.head._1.toByteVector shouldBe newMaxKey.toKey.toByteVector
        fromLessThanMinTenElemsNew.size shouldBe 10
        fromLessThanMinTenElemsNew.head._1.toByteVector shouldBe minKey.toKey.toByteVector

      }

    }

  }

  /* util methods */

  private def createBTreeClient(clientState: Option[ClientState] = None): MerkleBTreeClient[String] = {
    val keyCrypt = NoOpCrypt.forString[Task]
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
      .add[Option[NodeId]]
      .add[None.type]
      .addCase(classOf[Leaf])
      .addCase(classOf[Branch])
      .build[Task]()

    import codecs._

    val Arity = 4
    val Alpha = 0.25F

    val tMap = new TrieMap[Array[Byte], Array[Byte]](MurmurHash3.arrayHashing, Equiv.fromComparator(BytesOrdering))
    val store = new BTreeBinaryStore[Task, NodeId, Node](
      new TrieMapKVStore[Task, Array[Byte], Array[Byte]](tMap),
      () ⇒ blobIdCounter.incrementAndGet()
    )
    new MerkleBTree(MerkleBTreeConfig(arity = Arity, alpha = Alpha), store, NodeOps(hasher))
  }

  private def wait[T](task: Task[T], time: FiniteDuration = 3.second)(implicit TS: TestScheduler): T = {
    val async = task.runAsync
    TS.tick(time)
    async.futureValue
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
