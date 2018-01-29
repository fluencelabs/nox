package fluence.dataset.node.storage

import fluence.btree.client.MerkleBTreeClient
import fluence.btree.client.MerkleBTreeClient.ClientState
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

        // todo concurrent intensive test case

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

  private def createClientDbDriver(dbName: String): ClientDatasetStorage[Task, String, User] = {
    new ClientDatasetStorage(
      createBTreeClient(),
      createStorageRpc(dbName),
      valueCrypt,
      hasher
    )
  }

  // todo use DatasetStorageRpc implementation with network transport
  private def createStorageRpc(dbName: String): DatasetStorageRpc[Task] = {
    createDatasetStorage(dbName)
  }

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

  override protected def afterEach(): Unit = {
    val conf = RocksDbConf.read()
    Path(conf.dataDir).deleteRecursively()
  }

}
