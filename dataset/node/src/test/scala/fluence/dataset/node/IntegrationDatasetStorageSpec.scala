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

package fluence.dataset.node

import cats.instances.try_._
import cats.~>
import cats.syntax.compose._
import cats.syntax.profunctor._
import com.typesafe.config.ConfigFactory
import fluence.btree.client.MerkleBTreeClient
import fluence.btree.client.MerkleBTreeClient.ClientState
import fluence.btree.core.Hash
import fluence.btree.protocol.BTreeRpc
import fluence.crypto.{Crypto, CryptoError, DumbCrypto}
import fluence.crypto.algorithm.Ecdsa
import fluence.crypto.hash.JdkCryptoHasher
import fluence.dataset.client.ClientDatasetStorage
import fluence.dataset.node.DatasetNodeStorage.DatasetChanged
import fluence.dataset.protocol.DatasetStorageRpc
import fluence.storage.rocksdb.{RocksDbConf, RocksDbStore}
import monix.eval.Task
import monix.execution.ExecutionModel
import monix.execution.atomic.Atomic
import monix.execution.schedulers.TestScheduler
import monix.reactive.Observable
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.time.{Milliseconds, Seconds, Span}
import org.scalatest.{BeforeAndAfterAll, BeforeAndAfterEach, Matchers, WordSpec}
import slogging.{LogLevel, LoggerConfig, PrintLoggerFactory}

import scala.concurrent.duration.{FiniteDuration, _}
import scala.language.higherKinds
import scala.reflect.io.Path
import scala.util.{Random, Try}

// todo test this callback onDatasetChange callback
class IntegrationDatasetStorageSpec extends WordSpec with Matchers with ScalaFutures with BeforeAndAfterEach with BeforeAndAfterAll {
  override implicit def patienceConfig: PatienceConfig = PatienceConfig(Span(1, Seconds), Span(250, Milliseconds))

  private val signAlgo = Ecdsa.signAlgo
  private val keyPair = signAlgo.generateKeyPair.unsafe(None)
  private val signer = signAlgo.signer(keyPair)

  case class User(name: String, age: Int)

  private implicit class Str2Hash(str: String) {
    def toHash: Hash = Hash(str.getBytes)
  }

  //  private val hasher = TestCryptoHasher
  private val hasher = JdkCryptoHasher.Sha256

  private val testHasher = hasher.rmap(Hash(_))

  private val key1 = "k0001"
  private val val1 = User("Rico", 31)
  private val key2 = "k0002"
  private val val2 = User("Bob", 32)
  private val key3 = "k0003"
  private val val3 = User("Alan", 33)
  private val key4 = "k0004"
  private val val4 = User("Peter", 34)
  private val key5 = "k0005"
  private val val5 = User("Sam", 35)

  private val rocksFactory = new RocksDbStore.Factory

  "put, get and range" should {
    "return error and recover client state" when {
      "decrypt failed" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val valueCryptWithError = DumbCrypto.cipherString compose Crypto.liftEitherB[User, String](
          user ⇒ Right(s"ENC[${user.name},${user.age}]"),
          string ⇒ {
            val pattern = "ENC\\[([^,]*),([^\\]]*)\\]".r
            val pattern(name, age) = string
            if (name == val2.name) {
              Left(CryptoError("Can't decrypt value")) // will fail when will get key2
            } else Right(User(name, age.toInt))
          }
        )

        val counter = Atomic(0L)
        val clientWithCorruption: ClientDatasetStorage[String, User] = new ClientDatasetStorage(
          "test0".getBytes(),
          0L,
          createBTreeClient(),
          createStorageRpcWithNetworkError("test0", _ ⇒ Task(counter.incrementAndGet())),
          keyCrypt,
          valueCryptWithError,
          testHasher
        )

        wait(clientWithCorruption.put(key1, val1)) shouldBe None
        wait(clientWithCorruption.put(key2, val2)) shouldBe None
        wait(clientWithCorruption.get(key1)) shouldBe Some(val1)
        wait(clientWithCorruption.range(key1, key1).toListL) should contain theSameElementsInOrderAs List(key1 → val1)

        // fail in get
        wait(clientWithCorruption.get(key2).failed).getMessage shouldBe "Can't decrypt value"
        wait(clientWithCorruption.get(key1)) shouldBe Some(val1)

        // fail in range
        wait(clientWithCorruption.range(key1, key2).toListL.failed).getMessage shouldBe "Can't decrypt value"
        wait(clientWithCorruption.range(key1, key1).toListL) should contain theSameElementsInOrderAs List(key1 → val1)

        // fail in put
        wait(clientWithCorruption.put(key3, val3).failed).getMessage shouldBe "some network error"
        wait(clientWithCorruption.get(key1)) shouldBe Some(val1)

        counter.get shouldBe 2L
      }

    }

    "save and return correct results" when {
      "get and range from empty tree" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val client = createClientDbDriver("test1")

        val getResult = wait(client.get("k0001"))
        getResult shouldBe None

        val rangeResult = wait(client.range(key1, key1).toListL)
        rangeResult shouldBe empty
      }

      "put one key, get and range one key from empty tree" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val client = createClientDbDriver("test2")

        val res1 = wait(client.get(key1))
        val res2 = wait(client.range(key1, key1).toListL)
        val res3 = wait(client.put(key1, val1))
        val res4 = wait(client.get(key1))
        val res5 = wait(client.get(key2))
        val res6 = wait(client.range(key1, key1).toListL)
        val res7 = wait(client.range(key2, key2).toListL)

        res1 shouldBe None
        res2 shouldBe empty
        res3 shouldBe None
        res4 shouldBe Some(val1)
        res5 shouldBe None
        res6 should contain only key1 → val1
        res7 shouldBe empty
      }

      "put many value in random order; get and range theirs" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val minKey = "k0001"
        val midKey = "k0256"
        val maxKey = "k0512"
        val absentKey = "k2048"

        val client = createClientDbDriver("test3")

        // insert 1024 unique values

        val putRes1 = wait(Task.gather(
          Random.shuffle(1 to 512).map(i ⇒ {
            client.put(f"k$i%04d", User(f"v$i%04d", i % 100))
          })
        ))

        putRes1 should have size 512
        putRes1 should contain only None

        // get some values
        wait(client.get(minKey)).get shouldBe User("v0001", 1)
        wait(client.get(midKey)).get shouldBe User("v0256", 56)
        wait(client.get(maxKey)).get shouldBe User("v0512", 12)
        wait(client.get(absentKey)) shouldBe None

        // get some range values

        val allKeys = wait(client.range(minKey, maxKey).toListL)
        allKeys.head shouldBe minKey → User("v0001", 1)
        allKeys.last shouldBe maxKey → User("v0512", 12)
        allKeys.size shouldBe 512
        checkOrder(allKeys)

        val fromMidTenEl = wait(client.range(midKey, maxKey).take(10).toListL)
        fromMidTenEl.head shouldBe midKey → User("v0256", 56)
        fromMidTenEl.last shouldBe "k0265" → User("v0265", 65)
        fromMidTenEl.size shouldBe 10
        checkOrder(fromMidTenEl)

        val fromMaxToInf = wait(client.range(maxKey, absentKey).toListL)
        fromMaxToInf.head shouldBe maxKey → User("v0512", 12)
        fromMaxToInf.size shouldBe 1

        val noOverlap = wait(client.range(absentKey, absentKey).toListL)
        noOverlap shouldBe empty

        // insert 1024 new and 1024 duplicated values
        val putRes2 = wait(Task.gather(
          Random.shuffle(1 to 1024)
            .map(i ⇒ client.put(f"k$i%04d", User(f"v$i%04d new", i % 100)))
        ))

        putRes2 should have size 1024
        putRes2.filter(_.isEmpty) should have size 512
        putRes2.filter(_.isDefined).map(_.get) should contain allElementsOf (1 to 512).map(i ⇒ User(f"v$i%04d", i % 100))

        // get some values
        wait(client.get(minKey)).get shouldBe User("v0001 new", 1)
        wait(client.get(midKey)).get shouldBe User("v0256 new", 56)
        wait(client.get(maxKey)).get shouldBe User("v0512 new", 12)
        wait(client.get(absentKey)) shouldBe None

        // get some range values

        val allKeysNew = wait(client.range(minKey, maxKey).toListL)
        allKeysNew.head shouldBe minKey → User("v0001 new", 1)
        allKeysNew.last shouldBe maxKey → User("v0512 new", 12)
        allKeysNew.size shouldBe 512
        checkOrder(allKeysNew)

        val fromMidTenElNew = wait(client.range(midKey, maxKey).take(10).toListL)
        fromMidTenElNew.head shouldBe midKey → User("v0256 new", 56)
        fromMidTenElNew.last shouldBe "k0265" → User("v0265 new", 65)
        fromMidTenElNew.size shouldBe 10
        checkOrder(fromMidTenElNew)

        val fromMaxToInfNew = wait(client.range(maxKey, absentKey).toListL)
        fromMaxToInfNew.head shouldBe maxKey → User("v0512 new", 12)
        fromMaxToInfNew.last shouldBe "k1024" → User("v1024 new", 24)
        fromMaxToInfNew.size shouldBe 513
        checkOrder(fromMaxToInfNew)

        val noOverlapNew = wait(client.range(absentKey, absentKey).toListL)
        noOverlapNew shouldBe empty

      }

      "concurrent intensive put and get" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val client = createClientDbDriver("test4")
        val N = 128
        val changesCounter = Atomic(0l)

        val result: Seq[Option[User]] = wait(Task.gather(
          Random.shuffle(1 to N).map(i ⇒ {
            // invoke N puts
            client.put(f"k$i%04d", User(f"v$i%04d", i % 100))
          }) ++
            Random.shuffle(1 to N * 2).flatMap(i ⇒ {
              // invoke 2N puts and 2N gets and 2N range queries
              client.put(f"k$i%04d", User(f"v$i%04d", i % 100)) +:
                client.get(f"k$i%04d") +:
                client.range(f"k$i%04d", f"k${i + 10}%04d").toListL.map(l ⇒ l.headOption.map(_._2)) +:
                Nil
            }) ++
            Random.shuffle(1 to N).map(i ⇒ {
              // invoke N gets
              client.get(f"k$i%04d")
            }) ++
            Random.shuffle(1 to N).map(i ⇒ {
              // invoke N ranges
              client.range(f"k$i%04d", f"k${i + 10}%04d").toListL.map(l ⇒ l.headOption.map(_._2))
            })
        ))

        result should have size 9 * N
        result.filter(_.isDefined) should contain allElementsOf (1 to N).map(i ⇒ { Some(User(f"v$i%04d", i % 100)) })

      }

    }
  }

  /* util methods */
  private val keyCrypt = DumbCrypto.cipherString
  private val valueCrypt = DumbCrypto.cipherString compose Crypto.liftB[User, String](
    user ⇒ s"ENC[${user.name},${user.age}]",
    string ⇒ {
      val pattern = "ENC\\[([^,]*),([^\\]]*)\\]".r
      val pattern(name, age) = string
      User(name, age.toInt)
    }
  )

  private def createDatasetNodeStorage(dbName: String, onChange: DatasetChanged ⇒ Task[Unit]): DatasetNodeStorage = {
    implicit def runId[F[_]]: F ~> F = new (F ~> F) { override def apply[A](fa: F[A]): F[A] = fa }
    DatasetNodeStorage[Task](dbName, rocksFactory, ConfigFactory.load(), hasher, onChange)
      .runAsync(monix.execution.Scheduler.Implicits.global).futureValue
  }

  private def createClientDbDriver(dbName: String, clientState: Option[ClientState] = None): ClientDatasetStorage[String, User] = {
    val fullName = makeUnique(dbName)
    new ClientDatasetStorage(
      fullName.getBytes(),
      0L,
      createBTreeClient(clientState),
      createStorageRpc(fullName),
      keyCrypt,
      valueCrypt,
      testHasher
    )
  }

  private def createStorageRpcWithNetworkError(
    dbName: String,
    counter: DatasetChanged ⇒ Task[Unit]
  ): DatasetStorageRpc[Task, Observable] = {
    val origin = createDatasetNodeStorage(makeUnique(dbName), counter)
    new DatasetStorageRpc[Task, Observable] {
      override def remove(
        datasetId: Array[Byte],
        version: Long,
        removeCallbacks: BTreeRpc.RemoveCallback[Task]
      ): Task[Option[Array[Byte]]] = {
        origin.remove(version, removeCallbacks)
      }
      override def put(
        datasetId: Array[Byte],
        version: Long,
        putCallback: BTreeRpc.PutCallbacks[Task],
        encryptedValue: Array[Byte]
      ): Task[Option[Array[Byte]]] = {
        if (new String(encryptedValue) == "ENC[Alan,33]") {
          Task.raiseError(new IllegalStateException("some network error"))
        } else {
          origin.put(version, putCallback, encryptedValue)
        }
      }
      override def get(
        datasetId: Array[Byte],
        version: Long,
        getCallbacks: BTreeRpc.SearchCallback[Task]
      ): Task[Option[Array[Byte]]] =
        origin.get(getCallbacks)

      override def range(
        datasetId: Array[Byte],
        version: Long,
        searchCallbacks: BTreeRpc.SearchCallback[Task]
      ): Observable[(Array[Byte], Array[Byte])] = {
        origin.range(searchCallbacks)
      }
    }
  }

  private def createStorageRpc(dbName: String): DatasetStorageRpc[Task, Observable] =
    new DatasetStorageRpc[Task, Observable] {
      private val storage = createDatasetNodeStorage(dbName, _ ⇒ Task.unit)

      override def remove(
        datasetId: Array[Byte],
        version: Long,
        removeCallbacks: BTreeRpc.RemoveCallback[Task]
      ): Task[Option[Array[Byte]]] =
        storage.remove(version, removeCallbacks)

      override def put(
        datasetId: Array[Byte],
        version: Long,
        putCallbacks: BTreeRpc.PutCallbacks[Task],
        encryptedValue: Array[Byte]
      ): Task[Option[Array[Byte]]] =
        storage.put(version, putCallbacks, encryptedValue)

      override def get(
        datasetId: Array[Byte],
        version: Long,
        getCallbacks: BTreeRpc.SearchCallback[Task]
      ): Task[Option[Array[Byte]]] =
        storage.get(getCallbacks)

      override def range(
        datasetId: Array[Byte],
        version: Long,
        searchCallbacks: BTreeRpc.SearchCallback[Task]
      ): Observable[(Array[Byte], Array[Byte])] = {
        storage.range(searchCallbacks)
      }
    }

  private def createBTreeClient(clientState: Option[ClientState] = None): MerkleBTreeClient[String] = {
    MerkleBTreeClient(
      clientState,
      keyCrypt,
      testHasher,
      signer
    )
  }

  private def wait[T](task: Task[T], time: FiniteDuration = 3.second)(implicit TS: TestScheduler): T = {
    val async = task.runAsync
    TS.tick(time)
    async.futureValue
  }

  private def checkOrder(list: List[(String, User)]): Unit =
    list should contain theSameElementsInOrderAs list.sortBy(_._1) // should be ascending order

  override protected def beforeEach(): Unit = {
    val conf = RocksDbConf.read[Try](ConfigFactory.load()).get
    Path(conf.dataDir).deleteRecursively()
  }

  override protected def afterEach(): Unit = {
    rocksFactory.close.unsafeRunSync()
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

  private def makeUnique(dbName: String) = s"${this.getClass.getSimpleName}_${dbName}_${new Random().nextInt}"

}
