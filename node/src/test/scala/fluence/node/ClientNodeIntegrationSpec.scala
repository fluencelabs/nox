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

package fluence.node

import java.io.IOException
import java.net.ServerSocket

import cats.data.EitherT
import cats.instances.option._
import cats.kernel.Monoid
import cats.~>
import com.typesafe.config.{ ConfigFactory, ConfigValueFactory }
import fluence.btree.client.MerkleBTreeClient.ClientState
import fluence.client.core.{ ClientServices, FluenceClient }
import fluence.client.grpc.ClientGrpcServices
import fluence.contract.BasicContract
import fluence.contract.client.Contracts
import fluence.crypto.SignAlgo
import fluence.crypto.algorithm.Ecdsa
import fluence.crypto.cipher.NoOpCrypt
import fluence.crypto.hash.{ CryptoHasher, JdkCryptoHasher, TestCryptoHasher }
import fluence.crypto.keypair.KeyPair
import fluence.contract.client.Contracts.NotFound
import fluence.dataset.client.{ ClientDatasetStorage, ClientDatasetStorageApi }
import fluence.dataset.grpc.DatasetStorageClient.ServerError
import fluence.dataset.grpc.DatasetStorageServer.ClientError
import fluence.dataset.protocol.DatasetStorageRpc
import fluence.kad.protocol.{ Contact, Key }
import fluence.kad.{ KademliaConf, KademliaMVar }
import fluence.transport.TransportSecurity
import fluence.transport.grpc.client.GrpcClient
import io.grpc.StatusRuntimeException
import monix.eval.Task
import monix.execution.Scheduler.Implicits.global
import monix.execution.{ CancelableFuture, Scheduler }
import monix.reactive.Observable
import org.scalactic.source.Position
import org.scalatest.concurrent.PatienceConfiguration.Timeout
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.time.{ Milliseconds, Seconds, Span }
import org.scalatest.{ BeforeAndAfterAll, Matchers, WordSpec }
import scodec.bits.ByteVector
import shapeless._
import slogging._

import scala.concurrent.Future
import scala.concurrent.duration._
import scala.language.higherKinds
import scala.reflect.io.Path
import scala.util.{ Failure, Random, Success, Try }

class ClientNodeIntegrationSpec extends WordSpec with Matchers with ScalaFutures with BeforeAndAfterAll {
  override implicit def patienceConfig: PatienceConfig = PatienceConfig(Span(5, Seconds), Span(250, Milliseconds))

  private val algo: SignAlgo = Ecdsa.signAlgo
  private val testHasher: CryptoHasher[Array[Byte], Array[Byte]] = JdkCryptoHasher.Sha256
  private val alternativeHasher: CryptoHasher[Array[Byte], Array[Byte]] = TestCryptoHasher
  private val keyCrypt = NoOpCrypt.forString[Task]
  private val valueCrypt = NoOpCrypt.forString[Task]

  import algo.checker

  private implicit def runId[F[_]]: F ~> F = new (F ~> F) {
    override def apply[A](fa: F[A]): F[A] = fa
  }

  private implicit def runTask[F[_]]: Task ~> Future = new (Task ~> Future) {
    override def apply[A](fa: Task[A]): Future[A] = fa.runAsync
  }

  private implicit def runFuture[F[_]]: Future ~> Task = new (Future ~> Task) {
    override def apply[A](fa: Future[A]): Task[A] = Task.fromFuture(fa)
  }

  private val config = ConfigFactory.load()

  private val dummyContact = Contact(
    "localhost", 80, KeyPair.Public(ByteVector("k".getBytes)), 0L, "gitHash", "sign"
  )

  "Node" should {
    "get an exception" when {

      "port is already used" in {
        val port = 6111
        val server = new ServerSocket(port)
        try {
          // run node on the busy port

          val result = FluenceNode.startNode(config = config
            .withValue("fluence.grpc.server.port", ConfigValueFactory.fromAnyRef(port))
            .withValue("fluence.network.acceptLocal", ConfigValueFactory.fromAnyRef(true)))
          Try(result.unsafeRunSync()).failed.get shouldBe a[IOException]

        } finally {
          server.close()
        }
      }

      "tries joins the Kademlia network via dead node" in {
        runNodes { servers ⇒
          val exception = servers.head._2.kademlia.join(Seq(dummyContact), 1).failed.taskValue
          exception shouldBe a[RuntimeException]
          exception should have message "Can't join any node among known peers"
        }
      }

    }

    "joins the Kademlia network" in {
      runNodes { servers ⇒
        makeKadNetwork(servers)
      }
    }

  }

  private val absentKey = "non-existent key"
  private val key1 = "_key1"
  private val key2 = "_key2"
  private val key3 = "_key3"
  private val val1 = "_value1"
  private val val2 = "_value2"
  private val val3 = "_value3"
  private val val1New = "_value1-new"

  "Client" should {
    "get an exception" when {

      "asks for non existing node" in {
        val client = ClientGrpcServices.build[Task](GrpcClient.builder)
        val kadClient = client(dummyContact).kademlia

        val result = kadClient.ping().failed.taskValue
        result shouldBe a[StatusRuntimeException]
        // todo there should be more describable exception appears like NetworkException or TimeoutException with cause
      }

      "can't find node by key" in {
        val client = ClientGrpcServices.build[Task](GrpcClient.builder)
        runNodes { servers ⇒
          val (kademliaClient, _) = createClientApi(servers.head._1, client)

          val resultContact = kademliaClient.findNode(Key.fromString[Task]("non-exists contract").taskValue, 5).taskValue
          resultContact shouldBe None
        }
      }

      "reads non existing contract" in {
        val client = ClientGrpcServices.build[Task](GrpcClient.builder)
        runNodes { servers ⇒
          val (_, contractsApi) = createClientApi(servers.head._1, client)

          val resultContact = contractsApi.find(Key.fromString[Task]("non-exists contract").taskValue).failed.taskValue
          resultContact shouldBe NotFound
        }
      }

      "there are not enough nodes for the contract" in {
        import fluence.contract.ops.ContractRead._
        import fluence.contract.ops.ContractWrite._

        val client = ClientGrpcServices.build[Task](GrpcClient.builder)
        val keyPair = algo.generateKeyPair[Task]().value.taskValue.right.get
        val kadKey = Key.fromKeyPair[Task](keyPair).taskValue
        val signer = algo.signer(keyPair)

        runNodes { servers ⇒
          val seedContact: Contact = makeKadNetwork(servers)
          val (_, contractsApi) = createClientApi(seedContact, client)
          val offer = BasicContract.offer[Task](kadKey, participantsRequired = 999, signer = signer).taskValue
          offer.checkOfferSeal[Task]().taskValue shouldBe true

          val result = contractsApi.allocate(offer, c ⇒ WriteOps[Task, BasicContract](c).sealParticipants(signer)).failed.taskValue
          result shouldBe Contracts.CantFindEnoughNodes(10)
        }
      }

      "clients sign is invalid for offer" in {
        import fluence.contract.ops.ContractRead._
        import fluence.contract.ops.ContractWrite._

        val client = ClientGrpcServices.build[Task](GrpcClient.builder)
        val keyPairValid = algo.generateKeyPair[Task]().value.taskValue.right.get
        val keyPairBad = algo.generateKeyPair[Task]().value.taskValue.right.get
        val kadKey = Key.fromKeyPair[Task](keyPairValid).taskValue
        val signerValid = algo.signer(keyPairValid)
        val signerBad = algo.signer(keyPairBad)

        runNodes { servers ⇒
          val seedContact = makeKadNetwork(servers)
          val (_, contractsApi) = createClientApi(seedContact, client)
          val offer = BasicContract.offer[Task](kadKey, participantsRequired = 4, signer = signerBad).taskValue
          offer.checkOfferSeal[Task]().taskValue shouldBe false
          val result = contractsApi.allocate(offer, c ⇒ WriteOps[Task, BasicContract](c).sealParticipants(signerValid)).failed.taskValue
          result shouldBe Contracts.IncorrectOfferContract
        }
      }

      "reads and writes from dataset without contracting" in {
        val newClient = ClientGrpcServices.build[Task](GrpcClient.builder)

        runNodes { servers ⇒
          val firstContact: Contact = servers.head._1
          val storageRpc = newClient(firstContact).datasetStorage
          val datasetStorage = createDatasetStorage("dummy dataset ******".getBytes, storageRpc)

          val getResponse = datasetStorage.get("request key").failed.taskValue
          getResponse shouldBe a[ServerError]
          getResponse.getMessage shouldBe "Can't create DatasetNodeStorage for datasetId=ZHVtbXkgZGF0YXNldCAqKioqKio="

          val putResponse = datasetStorage.put("key", "value").failed.taskValue
          putResponse shouldBe a[ServerError]
          putResponse.getMessage shouldBe "Can't create DatasetNodeStorage for datasetId=ZHVtbXkgZGF0YXNldCAqKioqKio="

          val rangeResponse = datasetStorage.range("start key", "end key").failed.headL.taskValue
          rangeResponse shouldBe a[ServerError]
          rangeResponse.getMessage shouldBe "Can't create DatasetNodeStorage for datasetId=ZHVtbXkgZGF0YXNldCAqKioqKio="

        }
      }

      "client and server use different types of hasher" in {
        runNodes ({ servers ⇒
          val keyPair = algo.generateKeyPair[Id]().value.right.get
          val seedContact = makeKadNetwork(servers)
          val fluence = createFluenceClient(seedContact)

          val datasetStorage = fluence.createNewContract(keyPair, 2, keyCrypt, valueCrypt).taskValue(Some(timeout(Span(5, Seconds))))
          val nonExistentKeyResponse = datasetStorage.get(absentKey).taskValue
          nonExistentKeyResponse shouldBe None

          val nonExistentRangeResponse = datasetStorage.range(absentKey, absentKey).toListL.taskValue
          nonExistentRangeResponse shouldBe empty
          // put new value
          val putKey1Response = datasetStorage.put(key1, val1).failed.taskValue
          putKey1Response shouldBe a[ClientError]
          putKey1Response.getMessage should startWith("Server 'put response' didn't pass verifying for state=PutStateImpl")
        }, serverHasher = alternativeHasher)
      }
    }

    "success allocate a contract" in {
      import fluence.contract.ops.ContractRead._
      import fluence.contract.ops.ContractWrite._

      // create client
      val client = ClientGrpcServices.build[Task](GrpcClient.builder)
      // create offer
      val keyPair = algo.generateKeyPair[Task]().value.taskValue.right.get
      val kadKey = Key.fromKeyPair[Task](keyPair).taskValue
      val signer = algo.signer(keyPair)
      val offer = BasicContract.offer[Task](kadKey, participantsRequired = 4, signer = signer).taskValue
      offer.checkOfferSeal[Task]().taskValue shouldBe true

      runNodes { servers ⇒
        val seedContact = makeKadNetwork(servers)
        val (_, contractsApi) = createClientApi(seedContact, client)

        val acceptedContract = contractsApi.allocate(offer, c ⇒ WriteOps[Task, BasicContract](c).sealParticipants(signer)).taskValue

        acceptedContract.participants.size shouldBe 4
        contractsApi.find(kadKey).taskValue shouldBe acceptedContract
      }
    }

    "success write and read from dataset" in {
      runNodes { servers ⇒
        val keyPair = algo.generateKeyPair[Id]().value.right.get
        val seedContact = makeKadNetwork(servers)
        val fluence = createFluenceClient(seedContact)

        val datasetStorage = fluence.createNewContract(keyPair, 2, keyCrypt, valueCrypt).taskValue
        verifyReadAndWrite(datasetStorage)
        verifyRangeQueries(datasetStorage)
      }
    }

    "reads and puts values to dataset, client are restarted and continue to reading and writing" in {

      runNodes { servers ⇒
        val keyPair = algo.generateKeyPair[Id]().value.right.get
        val seedContact = makeKadNetwork(servers)
        val fluence1 = createFluenceClient(seedContact)

        val datasetStorage1 = fluence1.createNewContract(keyPair, 2, keyCrypt, valueCrypt).taskValue
        verifyReadAndWrite(datasetStorage1)

        // create new client (restart imitation)
        val fluence2 = createFluenceClient(seedContact)

        val datasetStorage2 = fluence2.getDataset(keyPair, keyCrypt, valueCrypt).taskValue.get
        val getKey1Result = datasetStorage2.get(key1).taskValue
        getKey1Result shouldBe Some(val1New)

        val putKey2Result = datasetStorage2.put(key3, val3).taskValue
        putKey2Result shouldBe None

        val getKey2Result = datasetStorage2.get(key3).taskValue
        getKey2Result shouldBe Some(val3)
      }

    }

    "reads and puts values to dataset, executor-node are restarted, client reconnect and continue to reading and writing" in {

      runNodes { servers ⇒
        // create client and write to db
        val keyPair = algo.generateKeyPair[Id]().value.right.get
        val seedContact = makeKadNetwork(servers)
        val grpcClient = ClientGrpcServices.build[Task](GrpcClient.builder)
        val (kademliaClient, contractsApi) = createClientApi(seedContact, grpcClient)
        @volatile var nodeCaptor: Contact = null
        val fluence = FluenceClient(
          kademliaClient,
          contractsApi,
          c ⇒ {
            nodeCaptor = c // capture executor node for shutdown
            grpcClient(c).datasetStorage
          },
          algo,
          testHasher
        )
        val datasetStorage = fluence.createNewContract(keyPair, 2, keyCrypt, valueCrypt).taskValue(Some(timeout(Span(5, Seconds))))

        verifyReadAndWrite(datasetStorage)

        // shutdown the executor-node and wait when it's up
        val server = servers.find { case (c, _) ⇒ c.publicKey == nodeCaptor.publicKey }.get._2
        shutdownNodeAndRestart(server) { _ ⇒

          val datasetStorageReconnected = fluence.getDataset(keyPair, keyCrypt, valueCrypt).taskValue(Some(timeout(Span(5, Seconds)))).get

          val getKey1Result = datasetStorageReconnected.get(key1).taskValue(Some(timeout(Span(1, Seconds))))
          getKey1Result shouldBe Some(val1New)

          val putKey2Result = datasetStorageReconnected.put(key3, val3).taskValue
          putKey2Result shouldBe None

          val getKey2Result = datasetStorageReconnected.get(key3).taskValue
          getKey2Result shouldBe Some(val3)
        }
      }
    }

  }

  private def shutdownNodeAndRestart(nodeExecutor: FluenceNode)(action: Contact ⇒ Unit): Unit = {
    val contact: Contact = nodeExecutor.contact.taskValue
    // restart
    val restarted = nodeExecutor.restart.unsafeRunSync()

    try {
      // start all nodes Grpc servers
      action(contact)
    } finally {
      restarted.stop.unsafeRunSync()
    }
  }

  /** Makes some reads and writes and check results */
  private def verifyReadAndWrite(datasetStorage: ClientDatasetStorageApi[Task, Observable, String, String]) = {
    // read non-existent value
    val nonExistentKeyResponse = datasetStorage.get(absentKey).taskValue
    nonExistentKeyResponse shouldBe None
    val nonExistentRangeKeyResponse = datasetStorage.range(absentKey, absentKey).toListL.taskValue
    nonExistentRangeKeyResponse shouldBe empty
    // put new value
    val putKey1Response = datasetStorage.put(key1, val1).taskValue
    putKey1Response shouldBe None
    // read new value
    val getKey1Response = datasetStorage.get(key1).taskValue
    getKey1Response shouldBe Some(val1)
    val rangeKey1Response = datasetStorage.range(key1, key1).toListL.taskValue
    rangeKey1Response should contain only key1 → val1
    // put new and override old value
    val putKey2Response = datasetStorage.put(key2, val2).taskValue
    putKey2Response shouldBe None
    val putKey1Again = datasetStorage.put(key1, val1New).taskValue
    putKey1Again shouldBe Some(val1)
    // read updated value
    val getUpdatedKey1Response = datasetStorage.get(key1).taskValue
    getUpdatedKey1Response shouldBe Some(val1New)
    val rangeUpdatedKey1Response = datasetStorage.range(key1, key1).toListL.taskValue
    rangeUpdatedKey1Response should contain only key1 → val1New
    val rangeAllResponse = datasetStorage.range(key1, key2).toListL.taskValue
    rangeAllResponse should contain theSameElementsInOrderAs List(key1 → val1New, key2 → val2)
  }

  /** Makes many writes and read with range */
  private def verifyRangeQueries(
    datasetStorage: ClientDatasetStorageApi[Task, Observable, String, String],
    numberOfKeys: Int = 256
  ): Unit = {

    val allRecords = Random.shuffle(1 to numberOfKeys).map(i ⇒ { f"k$i%04d" → f"v$i%04d" })

    // invoke numberOfKeys puts
    Task.sequence(allRecords.map { case (k, v) ⇒ datasetStorage.put(k, v) }).taskValue(Some(timeout(Span(10, Seconds))))

    val firstKey = "k0001"
    val midKey = f"k${numberOfKeys / 2}%04d"
    val lastKey = f"k$numberOfKeys%04d"

    val firstVal = "v0001"
    val midVal = f"v${numberOfKeys / 2}%04d"
    val lastVal = f"v$numberOfKeys%04d"

    val notOverlapRange1 = datasetStorage.range("a", "b").toListL.taskValue
    notOverlapRange1 shouldBe empty

    val notOverlapRange2 = datasetStorage.range("x", "y").toListL.taskValue
    notOverlapRange2 shouldBe empty

    val firstEl = datasetStorage.range(firstKey, firstKey).toListL.taskValue
    firstEl should contain theSameElementsInOrderAs List(firstKey → firstVal)

    val fromFirstToEnd = datasetStorage.range(firstKey, lastKey).toListL.taskValue
    fromFirstToEnd.head shouldBe firstKey → firstVal
    fromFirstToEnd.last shouldBe lastKey → lastVal
    fromFirstToEnd should contain theSameElementsInOrderAs allRecords.sortBy(_._1)
    checkOrder(fromFirstToEnd)

    val fromFirstToEnd2 = datasetStorage.range("a", "z").toListL.taskValue
    fromFirstToEnd2.head shouldBe firstKey → firstVal
    fromFirstToEnd2.last shouldBe lastKey → lastVal
    fromFirstToEnd2 should contain theSameElementsInOrderAs allRecords.sortBy(_._1)
    checkOrder(fromFirstToEnd2)

    val fromFirstToMid = datasetStorage.range(firstKey, midKey).toListL.taskValue
    fromFirstToMid.head shouldBe firstKey → firstVal
    fromFirstToMid.last shouldBe midKey → midVal
    fromFirstToMid.size shouldBe numberOfKeys / 2
    checkOrder(fromFirstToMid)

    val N = 10
    val fromMidNRecords = datasetStorage.range(midKey, lastKey).take(N).toListL.taskValue
    fromMidNRecords.head shouldBe midKey → midVal
    fromMidNRecords.last shouldBe f"k${(numberOfKeys / 2) + N - 1}%04d" → f"v${(numberOfKeys / 2) + N - 1}%04d"
    fromMidNRecords.size shouldBe N
    checkOrder(fromMidNRecords)
  }

  private def createClientApi[T <: HList](
    seedContact: Contact,
    client: Contact ⇒ ClientServices[Task, BasicContract, Contact]
  ) = {
    val conf = KademliaConf(100, 10, 2, 5.seconds)
    val clKey = Monoid.empty[Key]
    val check = TransportSecurity.canBeSaved[Task](clKey, acceptLocal = true)
    val kademliaRpc = client(_: Contact).kademlia
    val kademliaClient = KademliaMVar.client(kademliaRpc, conf, check)

    kademliaClient.join(Seq(seedContact), 2).taskValue

    (kademliaClient, new Contracts[Task, BasicContract, Contact](
      maxFindRequests = 10,
      maxAllocateRequests = _ ⇒ 20,
      kademlia = kademliaClient,
      cacheRpc = client(_).contractsCache,
      allocatorRpc = client(_).contractAllocator
    ))
  }

  private def makeKadNetwork(servers: Map[Contact, FluenceNode]) = {
    val firstContact: Contact = servers.head._1
    val lastContact = servers.last._1
    servers.foreach { _._2.kademlia.join(Seq(firstContact, lastContact), 8).taskValue }
    firstContact
  }

  private def createDatasetStorage(
    datasetId: Array[Byte],
    grpc: DatasetStorageRpc[Task, Observable],
    merkleRoot: Option[Array[Byte]] = None
  ): ClientDatasetStorage[String, String] = {
    val value1: Option[ClientState] = merkleRoot.map(mr ⇒ ClientState(ByteVector(mr)))
    ClientDatasetStorage(
      datasetId,
      testHasher,
      grpc,
      keyCrypt,
      valueCrypt,
      value1
    )
  }

  /**
   * Test utility method for running N nodes and shutting down after all.
   * @param action An action to executing
   */
  private def runNodes(
    action: Map[Contact, FluenceNode] ⇒ Unit,
    numberOfNodes: Int = 10,
    serverHasher: CryptoHasher[Array[Byte], Array[Byte]] = testHasher
  ): Unit = {

    var servers: Seq[FluenceNode] = Nil

    try {
      // start all nodes Grpc servers
      servers = (0 until numberOfNodes).map { n ⇒
        val port = 6112 + n
        // storage root directory, keys directory, etc should be modified to isolate nodes
        FluenceNode.startNode(
          algo,
          serverHasher,
          config
            .withValue("fluence.grpc.server.port", ConfigValueFactory.fromAnyRef(port))
            .withValue("fluence.network.acceptLocal", ConfigValueFactory.fromAnyRef(true))
            .withValue(ContractsCacheConf.ConfigPath + "dirName", ConfigValueFactory.fromAnyRef("node_cache_" + n))
            .withValue("fluence.directory", ConfigValueFactory.fromAnyRef(System.getProperty("java.io.tmpdir") + makeUnique(s"/testnode-$n")))
            //override for some value with no file for new key pair
            .withValue("fluence.keys.keyPath", ConfigValueFactory.fromAnyRef(System.getProperty("java.io.tmpdir") + makeUnique(s"/testnode-kp-$n")))
            .withValue("fluence.storage.rocksDb.dataDir", ConfigValueFactory.fromAnyRef(System.getProperty("java.io.tmpdir") + makeUnique(s"/rocksdb-ds-$n")))
            .withValue("fluence.dataset.node.dataDir", ConfigValueFactory.fromAnyRef(System.getProperty("java.io.tmpdir") + makeUnique(s"/rocksdb-ds-$n/btree_idx")))
            .withValue("fluence.dataset.node.indexDir", ConfigValueFactory.fromAnyRef(System.getProperty("java.io.tmpdir") + makeUnique(s"/rocksdb-ds-$n/blob_data")))
        ).unsafeRunSync()
      }

      action(servers.map(fn ⇒ fn.contact.taskValue → fn).toMap)
    } finally {
      // shutting down all nodes and close() RocksDb instances
      servers.foreach { s ⇒
        s.stop.unsafeRunSync()

        // clean all rockDb data from disk
        if (s.config.getString("fluence.storage.rocksDb.dataDir").startsWith(System.getProperty("java.io.tmpdir")))
          Path(s.config.getString("fluence.storage.rocksDb.dataDir")).deleteRecursively()
        if (s.config.getString("fluence.directory").startsWith(System.getProperty("java.io.tmpdir")))
          Path(s.config.getString("fluence.directory")).deleteRecursively()
        if (s.config.getString("fluence.keys.keyPath").startsWith(System.getProperty("java.io.tmpdir")))
          Path(s.config.getString("fluence.keys.keyPath")).deleteRecursively()
      }

    }

  }

  private def createFluenceClient(seed: Contact): FluenceClient = {
    val grpcClient = ClientGrpcServices.build[Task](GrpcClient.builder)
    val (kademliaClient, contractsApi) = createClientApi(seed, grpcClient)
    FluenceClient(kademliaClient, contractsApi, grpcClient(_).datasetStorage, algo, testHasher)
  }

  private implicit class WaitTask[T](task: Task[T]) {
    def taskValue(implicit s: Scheduler, pos: Position): T = taskValue(None)(s, pos)
    def taskValue(timeoutOp: Option[Timeout] = None)(implicit s: Scheduler, pos: Position): T = {
      val future: CancelableFuture[T] = task.runAsync(s)
      future.onComplete {
        case Success(_) ⇒ ()
        case Failure(exception) ⇒
          println(Console.RED + s"TASK ERROR: $exception")
          exception.printStackTrace()
      }(s)
      future.futureValue(timeoutOp.getOrElse(timeout(implicitly[PatienceConfig].timeout)))
    }
  }

  private implicit class OptionEitherTExtractor[A, B](et: EitherT[Option, A, B]) {
    def eitherValue: B = et.value.get.right.get
  }

  private def checkOrder(list: List[(String, String)]): Unit =
    list should contain theSameElementsInOrderAs list.sortBy(_._1) // should be ascending order

  override protected def beforeAll(): Unit = {
    LoggerConfig.factory = PrintLoggerFactory
    LoggerConfig.level = LogLevel.OFF
    super.beforeAll()
  }

  override protected def afterAll(): Unit = {
    super.afterAll()
    Path(config.getString("fluence.storage.rocksDb.dataDir")).deleteRecursively()
    LoggerConfig.level = LogLevel.OFF
  }

  private def makeUnique(dbName: String) = s"${dbName}_${this.getClass.getSimpleName}_${new Random().nextInt}"

}

