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
import java.net.{ InetAddress, ServerSocket }

import cats.data.EitherT
import cats.instances.option._
import cats.kernel.Monoid
import cats.~>
import com.typesafe.config.{ ConfigFactory, ConfigValueFactory }
import fluence.btree.client.MerkleBTreeClient.ClientState
import fluence.client.{ AuthorizedClient, ClientComposer, FluenceClient }
import fluence.crypto.SignAlgo
import fluence.crypto.algorithm.Ecdsa
import fluence.crypto.cipher.NoOpCrypt
import fluence.crypto.hash.{ CryptoHasher, JdkCryptoHasher, TestCryptoHasher }
import fluence.crypto.keypair.KeyPair
import fluence.dataset.BasicContract
import fluence.dataset.client.Contracts.NotFound
import fluence.dataset.client.{ ClientDatasetStorage, ClientDatasetStorageApi, Contracts }
import fluence.dataset.grpc.DatasetStorageClient.ServerError
import fluence.dataset.grpc.DatasetStorageServer.ClientError
import fluence.dataset.protocol.storage.DatasetStorageRpc
import fluence.dataset.protocol.{ ContractAllocatorRpc, ContractsCacheRpc }
import fluence.kad.protocol.{ Contact, KademliaRpc, Key }
import fluence.kad.{ KademliaConf, KademliaMVar }
import fluence.transport.TransportSecurity
import fluence.transport.grpc.client.GrpcClient
import io.grpc.StatusRuntimeException
import monix.eval.Task
import monix.execution.Scheduler.Implicits.global
import monix.execution.{ CancelableFuture, Scheduler }
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
import scala.util.{ Failure, Success, Try }

class ClientNodeIntegrationSpec extends WordSpec with Matchers with ScalaFutures with BeforeAndAfterAll {
  override implicit def patienceConfig: PatienceConfig = PatienceConfig(Span(5, Seconds), Span(250, Milliseconds))

  private val algo: SignAlgo = Ecdsa.signAlgo
  private val testHasher: CryptoHasher[Array[Byte], Array[Byte]] = TestCryptoHasher
  private val testpass: Array[Char] = "testpass".toCharArray

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
    InetAddress.getByName("localhost"), 80, KeyPair.Public(ByteVector("k".getBytes)), 0L, "gitHash", "sign"
  )

  "Node" should {
    "get an exception" when {

      "port is already used" in {
        val port = 6111
        val server = new ServerSocket(port)
        try {
          // run node on the busy port

          // TODO: check if rocksdb is closed properly
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

  "Client" should {
    "get an exception" when {

      "asks for non existing node" in {
        val client = ClientComposer.grpc[Task](GrpcClient.builder)
        val kadClient = client.service[KademliaRpc[Task, Contact]](dummyContact)

        val result = kadClient.ping().failed.taskValue
        result shouldBe a[StatusRuntimeException]
        // todo there should be more describable exception appears like NetworkException or TimeoutException with cause
      }

      "can't find node by key" in {
        val client = ClientComposer.grpc[Task](GrpcClient.builder)
        runNodes { servers ⇒
          val (kademliaClient, _) = createClientApi(servers.head._1, client)

          val resultContact = kademliaClient.findNode(Key.fromString[Task]("non-exists contract").taskValue, 5).taskValue
          resultContact shouldBe None
        }
      }

      "reads non existing contract" in {
        val client = ClientComposer.grpc[Task](GrpcClient.builder)
        runNodes { servers ⇒
          val (_, contractsApi) = createClientApi(servers.head._1, client)

          val resultContact = contractsApi.find(Key.fromString[Task]("non-exists contract").taskValue).failed.taskValue
          resultContact shouldBe NotFound
        }
      }

      "there are not enough nodes for the contract" in {
        import fluence.dataset.contract.ContractRead._
        import fluence.dataset.contract.ContractWrite._

        val client = ClientComposer.grpc[Task](GrpcClient.builder)
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
        import fluence.dataset.contract.ContractRead._
        import fluence.dataset.contract.ContractWrite._

        val client = ClientComposer.grpc[Task](GrpcClient.builder)
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
        val newClient = ClientComposer.grpc[Task](GrpcClient.builder)

        runNodes { servers ⇒
          val firstContact: Contact = servers.head._1
          val storageRpc = newClient.service[DatasetStorageRpc[Task]](firstContact)
          val datasetStorage = createDatasetStorage("dummy dataset ******".getBytes, storageRpc)

          val getResponse = datasetStorage.get("request key").failed.taskValue
          getResponse shouldBe a[ServerError]
          getResponse.getMessage shouldBe "Can't get DatasetNodeStorage for datasetId=ZHVtbXkgZGF0YXNldCAqKioqKio="

          val putResponse = datasetStorage.put("key", "value").failed.taskValue
          getResponse shouldBe a[ServerError]
          getResponse.getMessage shouldBe "Can't get DatasetNodeStorage for datasetId=ZHVtbXkgZGF0YXNldCAqKioqKio="

        }
      }

      "client and server use different types of hasher" in {
        runNodes ({ servers ⇒
          val client = AuthorizedClient.generateNew[Option](algo, testpass).eitherValue
          val seedContact = makeKadNetwork(servers)
          val fluence = createFluenceClient(seedContact)

          val datasetStorage = fluence.getOrCreateDataset(client).taskValue(Some(timeout(Span(5, Seconds))))

          val nonExistentKeyResponse = datasetStorage.get("non-existent key").taskValue
          nonExistentKeyResponse shouldBe None
          // put new value
          val putKey1Response = datasetStorage.put("key1", "value1").failed.taskValue
          putKey1Response shouldBe a[ClientError]
          putKey1Response.getMessage should startWith("Server 'put response' didn't pass verifying for state=PutStateImpl")
        }, serverHasher = JdkCryptoHasher.Sha256)
      }
    }

    "success allocate a contract" in {
      import fluence.dataset.contract.ContractRead._
      import fluence.dataset.contract.ContractWrite._

      // create client
      val client = ClientComposer.grpc[Task](GrpcClient.builder)
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
        val client = AuthorizedClient.generateNew[Option](algo, testpass).eitherValue
        val seedContact = makeKadNetwork(servers)
        val fluence = createFluenceClient(seedContact)

        val datasetStorage = fluence.getOrCreateDataset(client).taskValue(Some(timeout(Span(5, Seconds))))
        verifyReadAndWrite(datasetStorage)
      }
    }

    "reads and puts values to dataset, client are restarted and continue to reading and writing" in {

      runNodes { servers ⇒
        val client = AuthorizedClient.generateNew[Option](algo, testpass).eitherValue
        val seedContact = makeKadNetwork(servers)
        val fluence1 = createFluenceClient(seedContact)

        val datasetStorage1 = fluence1.getOrCreateDataset(client).taskValue
        verifyReadAndWrite(datasetStorage1)

        // create new client (restart imitation)
        val fluence2 = createFluenceClient(seedContact)

        val datasetStorage2 = fluence2.getOrCreateDataset(client).taskValue
        val getKey1Result = datasetStorage2.get("key1").taskValue
        getKey1Result shouldBe Some("value1-NEW")

        val putKey2Result = datasetStorage2.put("key2", "value2").taskValue
        putKey2Result shouldBe None

        val getKey2Result = datasetStorage2.get("key2").taskValue
        getKey2Result shouldBe Some("value2")
      }

    }

    "reads and puts values to dataset, executor-node are restarted, client reconnect and continue to reading and writing" in {

      runNodes { servers ⇒
        // create client and write to db
        val client = AuthorizedClient.generateNew[Option](algo, testpass).eitherValue
        val seedContact = makeKadNetwork(servers)
        val grpcClient = ClientComposer.grpc[Task](GrpcClient.builder)
        val (kademliaClient, contractsApi) = createClientApi(seedContact, grpcClient)
        @volatile var nodeCaptor: Contact = null
        val fluence = FluenceClient(
          kademliaClient,
          contractsApi,
          c ⇒ {
            nodeCaptor = c // capture executor node for shutdown
            grpcClient.service[DatasetStorageRpc[Task]](c)
          },
          algo,
          testHasher,
          config
        )
        val datasetStorage = fluence.getOrCreateDataset(client).taskValue(Some(timeout(Span(5, Seconds))))

        verifyReadAndWrite(datasetStorage)

        // shutdown the executor-node and wait when it's up
        val server = servers.find { case (c, _) ⇒ c.publicKey == nodeCaptor.publicKey }.get._2
        shutdownNodeAndRestart(server) { _ ⇒

          val datasetStorageReconnected = fluence.getOrCreateDataset(client).taskValue(Some(timeout(Span(5, Seconds))))

          val getKey1Result = datasetStorageReconnected.get("key1").taskValue
          getKey1Result shouldBe Some("value1-NEW")

          val putKey2Result = datasetStorageReconnected.put("key2", "value2").taskValue
          putKey2Result shouldBe None

          val getKey2Result = datasetStorageReconnected.get("key2").taskValue
          getKey2Result shouldBe Some("value2")
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
  private def verifyReadAndWrite(datasetStorage: ClientDatasetStorageApi[Task, String, String]) = {
    // read non-existent value
    val nonExistentKeyResponse = datasetStorage.get("non-existent key").taskValue
    nonExistentKeyResponse shouldBe None
    // put new value
    val putKey1Response = datasetStorage.put("key1", "value1").taskValue
    putKey1Response shouldBe None
    // read new value
    val getKey1Response = datasetStorage.get("key1").taskValue
    getKey1Response shouldBe Some("value1")
    // override value
    val overrideResponse = datasetStorage.put("key1", "value1-NEW").taskValue
    overrideResponse shouldBe Some("value1")
    // read updated value
    val getUpdatedKey1Response = datasetStorage.get("key1").taskValue
    getUpdatedKey1Response shouldBe Some("value1-NEW")
  }

  /**
   * Creates client, makes kademlia network, allocate contract, get node executor and prepare datasetStorage
   * @return [[ClientDatasetStorage]] ready for working
   */
  private def createClientAndGetDatasetStorage(servers: Map[Contact, FluenceNode]) = {
    import fluence.dataset.contract.ContractWrite._

    // create client
    val client = ClientComposer.grpc[Task](GrpcClient.builder)
    // create offer
    val keyPair = algo.generateKeyPair[Task]().value.taskValue.right.get
    val kadKey = Key.fromKeyPair[Task](keyPair).taskValue
    val signer = algo.signer(keyPair)
    val offer = BasicContract.offer[Task](kadKey, participantsRequired = 4, signer = signer).taskValue

    val seedContact = makeKadNetwork(servers)
    val (_, contractsApi) = createClientApi(seedContact, client)
    val acceptedContract = contractsApi.allocate(offer, c ⇒ WriteOps[Task, BasicContract](c).sealParticipants(signer)).taskValue

    val (nodeKey, _) = acceptedContract.participants.head
    // we won't find node address by nodeId, cause service FluenceClient is not ready yet, it'll be later
    val (nodeContact, nodeComposer): (Contact, FluenceNode) =
      servers.find { case (c, _) ⇒ Key.fromPublicKey[Task](c.publicKey).taskValue == nodeKey }.get

    val storageRpc = client.service[DatasetStorageRpc[Task]](nodeContact)
    val datasetStorage = createDatasetStorage(acceptedContract.id.id, storageRpc)

    datasetStorage → (nodeComposer, client, acceptedContract)
  }

  private def createClientApi[T <: HList](seedContact: Contact, client: GrpcClient[T])(
    implicit
    s1: ops.hlist.Selector[T, ContractsCacheRpc[Task, BasicContract]],
    s2: ops.hlist.Selector[T, ContractAllocatorRpc[Task, BasicContract]],
    s3: ops.hlist.Selector[T, KademliaRpc[Task, Contact]]
  ) = {

    val conf = KademliaConf(100, 10, 2, 5.seconds)
    val clKey = Monoid.empty[Key]
    val check = TransportSecurity.canBeSaved[Task](clKey, acceptLocal = true)
    val kademliaRpc = client.service[KademliaRpc[Task, Contact]] _
    val kademliaClient = KademliaMVar.client(kademliaRpc, conf, check)

    kademliaClient.join(Seq(seedContact), 2).taskValue

    (kademliaClient, new Contracts[Task, BasicContract, Contact](
      maxFindRequests = 10,
      maxAllocateRequests = _ ⇒ 20,
      kademlia = kademliaClient,
      cacheRpc = contact ⇒ client.service[ContractsCacheRpc[Task, BasicContract]](contact),
      allocatorRpc = contact ⇒ client.service[ContractAllocatorRpc[Task, BasicContract]](contact)
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
    grpc: DatasetStorageRpc[Task],
    merkleRoot: Option[Array[Byte]] = None
  ): ClientDatasetStorage[String, String] = {
    val value1: Option[ClientState] = merkleRoot.map(ClientState)
    ClientDatasetStorage(
      datasetId,
      testHasher,
      grpc,
      NoOpCrypt.forString,
      NoOpCrypt.forString,
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
        // TODO: storage root directory, keys directory, etc should be modified to isolate nodes
        FluenceNode.startNode(
          algo,
          serverHasher,
          config
            .withValue("fluence.grpc.server.port", ConfigValueFactory.fromAnyRef(port))
            .withValue("fluence.network.acceptLocal", ConfigValueFactory.fromAnyRef(true))
            .withValue("fluence.contract.cacheDirName", ConfigValueFactory.fromAnyRef("node_cache_" + n))
            .withValue("fluence.directory", ConfigValueFactory.fromAnyRef(System.getProperty("java.io.tmpdir") + "/testnode-" + n))
            //override for some value with no file for new key pair
            .withValue("fluence.keyPath", ConfigValueFactory.fromAnyRef(System.getProperty("java.io.tmpdir") + "/testnode-kp-" + n))
            .withValue("fluence.storage.rocksDb.dataDir", ConfigValueFactory.fromAnyRef(System.getProperty("java.io.tmpdir") + "/rocksdb-ds-" + n))

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
        if (s.config.getString("fluence.keyPath").startsWith(System.getProperty("java.io.tmpdir")))
          Path(s.config.getString("fluence.keyPath")).deleteRecursively()
      }

    }

  }

  private def createFluenceClient(seed: Contact): FluenceClient = {
    val grpcClient = ClientComposer.grpc[Task](GrpcClient.builder)
    val (kademliaClient, contractsApi) = createClientApi(seed, grpcClient)
    FluenceClient(kademliaClient, contractsApi, grpcClient.service[DatasetStorageRpc[Task]], algo, testHasher, config)
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

  override protected def beforeAll(): Unit = {
    LoggerConfig.factory = PrintLoggerFactory
    LoggerConfig.level = LogLevel.WARN
    super.beforeAll()
  }

  override protected def afterAll(): Unit = {
    super.afterAll()
    Path(config.getString("fluence.storage.rocksDb.dataDir")).deleteRecursively()
    LoggerConfig.level = LogLevel.OFF
  }

}

