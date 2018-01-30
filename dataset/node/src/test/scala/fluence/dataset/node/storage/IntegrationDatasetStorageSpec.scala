package fluence.dataset.node.storage

import fluence.btree.client.MerkleBTreeClient
import fluence.btree.client.MerkleBTreeClient.ClientState
import fluence.btree.protocol.BTreeRpc
import fluence.crypto.cipher.NoOpCrypt
import fluence.crypto.hash.JdkCryptoHasher
import fluence.dataset.client.ClientDatasetStorage
import fluence.dataset.protocol.storage.DatasetStorageRpc
import fluence.storage.rocksdb.RocksDbConf
import monix.eval.Task
import monix.execution.ExecutionModel
import monix.execution.atomic.Atomic
import monix.execution.schedulers.TestScheduler
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.time.{ Milliseconds, Seconds, Span }
import org.scalatest.{ BeforeAndAfterEach, Matchers, WordSpec }

import scala.concurrent.duration.{ FiniteDuration, _ }
import scala.reflect.io.Path
import scala.util.Random

class IntegrationDatasetStorageSpec extends WordSpec with Matchers with ScalaFutures with BeforeAndAfterEach {

  case class User(name: String, age: Int)

  private val blobIdCounter = Atomic(0L)
  private val hasher = JdkCryptoHasher.Sha256
  //  private val hasher = TestCryptoHasher

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

  "put and get" should {
    "return error and recover client state" when {
      "data corruption appears in get methods" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val valueCryptWithCorruption = NoOpCrypt[User](
          user ⇒ s"ENC[${user.name},${user.age}]".getBytes(),
          bytes ⇒ {
            val pattern = "ENC\\[([^,]*),([^\\]]*)\\]".r
            val pattern(name, age) = new String(bytes)
            if (name == val2.name) {
              throw new IllegalStateException("Can't decrypt value") // will fail when will get key2
            }
            User(name, age.toInt)
          }
        )

        val clientWithCorruption: ClientDatasetStorage[String, User] = new ClientDatasetStorage(
          createBTreeClient(),
          createStorageRpcWithNetworkError("test0"),
          valueCryptWithCorruption,
          hasher
        )

        wait(clientWithCorruption.put(key1, val1)) shouldBe None
        wait(clientWithCorruption.put(key2, val2)) shouldBe None
        wait(clientWithCorruption.get(key1)) shouldBe Some(val1)

        // fail in get
        wait(clientWithCorruption.get(key2).failed).getMessage shouldBe "Can't decrypt value"
        wait(clientWithCorruption.get(key1)) shouldBe Some(val1)

        // fail in put
        wait(clientWithCorruption.put(key3, val3).failed).getMessage shouldBe "some network error"
        wait(clientWithCorruption.get(key1)) shouldBe Some(val1)

      }

    }

    "save and return correct results" when {
      "get from empty tree" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val client = createClientDbDriver("test1")

        val result = wait(client.get("k0001"))
        result shouldBe None

      }

      "put key1, get key1 from empty tree" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)
        val client = createClientDbDriver("test2")

        val res1 = wait(client.get(key1))
        val res2 = wait(client.put(key1, val1))
        val res3 = wait(client.get(key1))
        val res4 = wait(client.get(key2))

        res1 shouldBe None
        res2 shouldBe None
        res3 shouldBe Some(val1)
        res4 shouldBe None
      }

      "put many value in random order and get theirs" in {
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

      }

      "concurrent intensive put and get" in {
        implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

        val client = createClientDbDriver("test4")
        val N = 128

        val result = wait(Task.gather(
          Random.shuffle(1 to N).map(i ⇒ {
            // only N puts
            client.put(f"k$i%04d", User(f"v$i%04d", i % 100))
          }) ++
            Random.shuffle(1 to N * 2).flatMap(i ⇒ {
              // 2N puts and 2N! gets
              client.put(f"k$i%04d", User(f"v$i%04d", i % 100)) +: client.get(f"k$i%04d") +: Nil
            }) ++
            Random.shuffle(1 to N).map(i ⇒ {
              // only N gets
              client.get(f"k$i%04d")
            })
        ))

        result should have size 6 * N
        result.filter(_.isDefined) should contain allElementsOf (1 to N).map(i ⇒ { Some(User(f"v$i%04d", i % 100)) })

      }

    }
  }

  /* util methods */

  private val keyCrypt = NoOpCrypt.forString
  private val valueCrypt = NoOpCrypt[User](
    user ⇒ s"ENC[${user.name},${user.age}]".getBytes(),
    bytes ⇒ {
      val pattern = "ENC\\[([^,]*),([^\\]]*)\\]".r
      val pattern(name, age) = new String(bytes)
      User(name, age.toInt)
    }
  )

  private def createDatasetStorage(dbName: String): DatasetStorage = {
    DatasetStorage(s"${this.getClass.getSimpleName}_$dbName", hasher, () ⇒ blobIdCounter.incrementAndGet())
      .toEither match {
        case Right(store) ⇒ store
        case Left(err)    ⇒ throw err
      }
  }

  private def createClientDbDriver(dbName: String): ClientDatasetStorage[String, User] = {
    new ClientDatasetStorage(
      createBTreeClient(),
      createStorageRpc(dbName),
      valueCrypt,
      hasher
    )
  }

  private def createStorageRpcWithNetworkError(dbName: String): DatasetStorageRpc[Task] = {
    val origin = createDatasetStorage(dbName)
    new DatasetStorageRpc[Task] {
      override def remove(removeCallbacks: BTreeRpc.RemoveCallback[Task]): Task[Option[Array[Byte]]] = {
        origin.remove(removeCallbacks)
      }
      override def put(putCallback: BTreeRpc.PutCallbacks[Task], encryptedValue: Array[Byte]): Task[Option[Array[Byte]]] = {
        if (new String(encryptedValue) == "ENC[Alan,33]") {
          Task.raiseError(new IllegalStateException("some network error"))
        } else {
          origin.put(putCallback, encryptedValue)
        }
      }
      override def get(getCallbacks: BTreeRpc.GetCallbacks[Task]): Task[Option[Array[Byte]]] =
        origin.get(getCallbacks)
    }
  }

  // todo use DatasetStorageRpc implementation with network transport
  private def createStorageRpc(dbName: String): DatasetStorageRpc[Task] =
    createDatasetStorage(dbName)

  private def createBTreeClient(clientState: Option[ClientState] = None): MerkleBTreeClient[String] = {
    MerkleBTreeClient(
      clientState,
      keyCrypt,
      hasher
    )
  }

  private def wait[T](task: Task[T], time: FiniteDuration = 3.second)(implicit TS: TestScheduler): T = {
    val async = task.runAsync
    TS.tick(time)
    async.futureValue
  }

  override protected def beforeEach(): Unit = {
    val conf = RocksDbConf.read()
    Path(conf.dataDir).deleteRecursively()
  }

}
