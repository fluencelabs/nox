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
import fluence.crypto.hash.{ CryptoHasher, TestCryptoHasher }
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.SignatureChecker
import fluence.dataset.BasicContract
import fluence.dataset.client.Contracts.NotFound
import fluence.dataset.client.{ ClientDatasetStorage, Contracts }
import fluence.dataset.protocol.storage.DatasetStorageRpc
import fluence.dataset.protocol.{ ContractAllocatorRpc, ContractsCacheRpc }
import fluence.kad.grpc.client.KademliaClient
import fluence.kad.protocol.{ Contact, Key }
import fluence.kad.{ KademliaConf, KademliaMVar }
import fluence.storage.rocksdb.RocksDbStore
import fluence.transport.TransportSecurity
import fluence.transport.grpc.client.GrpcClient
import io.grpc.StatusRuntimeException
import monix.eval.Task
import monix.execution.Scheduler.Implicits.global
import monix.execution.{ CancelableFuture, Scheduler }
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
import scala.util.{ Failure, Success }

class ClientNodeIntegrationSpec extends WordSpec with Matchers with ScalaFutures with BeforeAndAfterAll {
  override implicit def patienceConfig: PatienceConfig = PatienceConfig(Span(1, Seconds), Span(250, Milliseconds))

  private val algo: SignAlgo = Ecdsa.signAlgo
  private val testHasher: CryptoHasher[Array[Byte], Array[Byte]] = TestCryptoHasher

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
          val contractsCacheStore = RocksDbStore[Task]("node_cache", config).taskValue
          val composer = NodeComposer.services(
            algo.generateKeyPair[Task]().value.taskValue.right.get,
            algo,
            config
              .withValue("fluence.transport.grpc.server.localPort", ConfigValueFactory.fromAnyRef(port))
              .withValue("fluence.transport.grpc.server.externalPort", ConfigValueFactory.fromAnyRef(null))
              .withValue("fluence.transport.grpc.server.acceptLocal", ConfigValueFactory.fromAnyRef(true)),
            contractsCacheStore
          )
          try {
            val result = composer.server.taskValue.start().failed.taskValue
            result shouldBe a[IOException]
          } finally {
            composer.server.foreach(_.shutdown(3.seconds)).futureValue
            contractsCacheStore.close()
          }
        } finally {
          server.close()
        }
      }

      "tries joins the Kademlia network via dead node" in {
        runNodes { servers ⇒
          val exception = servers.head._2.services.taskValue.kademlia.join(Seq(dummyContact), 1).failed.taskValue
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
        val kadClient = client.service[KademliaClient[Task]](dummyContact)

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
          offer.checkOfferSeal[Task](algo.checker).taskValue shouldBe true

          val result = contractsApi.allocate(offer, c ⇒ WriteOps[Task, BasicContract](c).sealParticipants(signer)).failed.taskValue
          result shouldBe Contracts.CantFindEnoughNodes(20)
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
          offer.checkOfferSeal[Task](algo.checker).taskValue shouldBe false
          val result = contractsApi.allocate(offer, c ⇒ WriteOps[Task, BasicContract](c).sealParticipants(signerValid)).failed.taskValue
          result shouldBe Contracts.IncorrectOfferContract
        }
      }

      "reads and writes from dataset without contracting" in {
        val newClient = ClientComposer.grpc[Task](GrpcClient.builder)

        runNodes { servers ⇒
          val firstContact: Contact = servers.head._1
          val storageRpc = newClient.service[DatasetStorageRpc[Task]](firstContact)
          val datasetStorage = createDatasetStorage("dummy dataset".getBytes, storageRpc)

          val getResponse = datasetStorage.get("request key").failed.taskValue
          getResponse shouldBe a[StatusRuntimeException]
          // todo transmit error from server: there should be DatasetNotFound exception (or something else) instead of StatusRuntimeException

          val putResponse = datasetStorage.put("key", "value").failed.taskValue
          // todo transmit error from server: there should be DatasetNotFound exception (or something else) instead of StatusRuntimeException
          putResponse shouldBe a[StatusRuntimeException]

        }
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
      offer.checkOfferSeal[Task](algo.checker).taskValue shouldBe true

      runNodes { servers ⇒
        val seedContact = makeKadNetwork(servers)
        val (_, contractsApi) = createClientApi(seedContact, client)

        val acceptedContract = contractsApi.allocate(offer, c ⇒ WriteOps[Task, BasicContract](c).sealParticipants(signer)).taskValue

        acceptedContract.participants.size shouldBe 4
        contractsApi.find(kadKey).taskValue shouldBe acceptedContract
      }
    }

    "success write and read from dataset" in {
      runNodes { servers: Map[Contact, NodeComposer] ⇒
        val client = AuthorizedClient.generateNew[Option](algo).eitherValue
        val seedContact = makeKadNetwork(servers)
        val fluence = createFluenceClient(seedContact)

        val datasetStorage = fluence.getOrCreateDataset(client).taskValue
        verifyReadAndWrite(datasetStorage)
      }
    }

    "reads and puts values to dataset, client are restarted and continue to reading and writing" in {

      runNodes { servers ⇒
        val client = AuthorizedClient.generateNew[Option](algo).eitherValue
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

    // todo finish, this test can't work correct yet
    "reads and puts values to dataset, executor-node are restarted, client reconnect and continue to reading and writing" ignore {

      runNodes { servers ⇒
        // create client and write to db
        val client = AuthorizedClient.generateNew[Option](algo).eitherValue
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
          testHasher
        )
        val datasetStorage = fluence.getOrCreateDataset(client).taskValue

        verifyReadAndWrite(datasetStorage)

        // shutdown the executor-node and wait when it's up
        val server = servers.find { case (c, _) ⇒ c.publicKey == nodeCaptor.publicKey }.get._2
        shutdownNodeAndRestart(server) { _ ⇒

          val datasetStorageReconnected = fluence.getOrCreateDataset(client).taskValue

          val getKey1Result = datasetStorageReconnected.get("key1").taskValue // todo finish, here is not worked yet
          getKey1Result shouldBe Some("value1-NEW")

          val putKey2Result = datasetStorageReconnected.put("key2", "value2").taskValue
          putKey2Result shouldBe None

          val getKey2Result = datasetStorageReconnected.get("key2").taskValue
          getKey2Result shouldBe Some("value2")
        }
      }
    }

  }

  private def shutdownNodeAndRestart(nodeExecutor: NodeComposer)(action: Contact ⇒ Unit): Unit = {
    val contact: Contact = nodeExecutor.server.taskValue.contact.taskValue
    // shutdown
    nodeExecutor.server.taskValue.shutdown(5.second)
    val contractCacheStoreField = classOf[NodeComposer].getDeclaredField("fluence$node$NodeComposer$$contractCacheStore")
    contractCacheStoreField.setAccessible(true)
    val store = contractCacheStoreField.get(nodeExecutor).asInstanceOf[RocksDbStore]
    store.close()

    // start node
    val port = contact.port
    val contractsCacheStore = RocksDbStore[Task]("node_cache_" + (contact.port - 6112), config).taskValue
    val composer = new NodeComposer(
      algo.generateKeyPair[Task]().value.taskValue.right.get,
      algo,
      config
        .withValue("fluence.transport.grpc.server.localPort", ConfigValueFactory.fromAnyRef(port))
        .withValue("fluence.transport.grpc.server.externalPort", ConfigValueFactory.fromAnyRef(null))
        .withValue("fluence.transport.grpc.server.acceptLocal", ConfigValueFactory.fromAnyRef(true)),
      contractsCacheStore,
      testHasher
    )

    try {
      // start all nodes Grpc servers
      composer.server.taskValue.start()
      action(contact)
    } finally {
      composer.server.taskValue.shutdown(3.second)
    }
  }

  /** Makes some reads and writes and check results */
  private def verifyReadAndWrite(datasetStorage: ClientDatasetStorage[String, String]) = {
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
  private def createClientAndGetDatasetStorage(servers: Map[Contact, NodeComposer]) = {
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
    val (nodeContact, nodeComposer): (Contact, NodeComposer) =
      servers.find { case (c, _) ⇒ Key.fromPublicKey[Task](c.publicKey).taskValue == nodeKey }.get

    val storageRpc = client.service[DatasetStorageRpc[Task]](nodeContact)
    val datasetStorage = createDatasetStorage(acceptedContract.id.id, storageRpc)

    datasetStorage → (nodeComposer, client, acceptedContract)
  }

  private def createClientApi[T <: HList](seedContact: Contact, client: GrpcClient[T])(
    implicit
    s1: ops.hlist.Selector[T, ContractsCacheRpc[Task, BasicContract]],
    s2: ops.hlist.Selector[T, ContractAllocatorRpc[Task, BasicContract]],
    s3: ops.hlist.Selector[T, KademliaClient[Task]]
  ) = {

    val conf = KademliaConf(100, 10, 2, 5.seconds)
    val clKey = Monoid.empty[Key]
    val check = TransportSecurity.canBeSaved[Task](clKey, acceptLocal = true)
    val kademliaRpc = client.service[KademliaClient[Task]] _
    val kademliaClient = KademliaMVar.client(kademliaRpc, conf, check)

    kademliaClient.join(Seq(seedContact), 2).taskValue

    (kademliaClient, new Contracts[Task, BasicContract, Contact](
      maxFindRequests = 10,
      maxAllocateRequests = _ ⇒ 20,
      checker = algo.checker,
      kademlia = kademliaClient,
      cacheRpc = contact ⇒ client.service[ContractsCacheRpc[Task, BasicContract]](contact),
      allocatorRpc = contact ⇒ client.service[ContractAllocatorRpc[Task, BasicContract]](contact)
    ))
  }

  private def makeKadNetwork(servers: Map[Contact, NodeComposer]) = {
    val firstContact: Contact = servers.head._1
    val lastContact = servers.last._1
    servers.foreach { _._2.services.flatMap(_.kademlia.join(Seq(firstContact, lastContact), 8)).taskValue }
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
  private def runNodes(action: Map[Contact, NodeComposer] ⇒ Unit, numberOfNodes: Int = 20): Unit = {

    val servers = (0 to numberOfNodes).map { n ⇒
      val port = 6112 + n
      val contractsCacheStore = RocksDbStore[Task]("node_cache_" + n, config).taskValue
      val composer = new NodeComposer(
        algo.generateKeyPair[Task]().value.taskValue.right.get,
        algo,
        config
          .withValue("fluence.transport.grpc.server.localPort", ConfigValueFactory.fromAnyRef(port))
          .withValue("fluence.transport.grpc.server.externalPort", ConfigValueFactory.fromAnyRef(null))
          .withValue("fluence.transport.grpc.server.acceptLocal", ConfigValueFactory.fromAnyRef(true)),
        contractsCacheStore,
        testHasher
      )

      composer.server.flatMap(_.contact).taskValue :: composer :: contractsCacheStore :: HNil
    }

    try {
      // start all nodes Grpc servers
      servers.foreach(_.tail.head.server.flatMap(_.start()).runAsync.futureValue)
      action(servers.map{ case c :: s :: tail ⇒ c → s }.toMap)
    } finally {
      // shutting down all nodes and close() RocksDb instances
      servers.foreach {
        case _ :: composer :: db :: HNil ⇒
          composer.server.foreach(_.shutdown(3.second))
          db.close()
      }
      // clean all rockDb data from disk
      Path(config.getString("fluence.node.storage.rocksDb.dataDir")).deleteRecursively()
    }

  }

  private def createFluenceClient(seed: Contact): FluenceClient = {
    val grpcClient = ClientComposer.grpc[Task](GrpcClient.builder)
    val (kademliaClient, contractsApi) = createClientApi(seed, grpcClient)
    FluenceClient(kademliaClient, contractsApi, grpcClient.service[DatasetStorageRpc[Task]], algo, testHasher)
  }

  private implicit class WaitTask[T](task: Task[T]) {
    def taskValue(implicit s: Scheduler): T = taskValue(None)(s)
    def taskValue(timeoutOp: Option[Timeout] = None)(implicit s: Scheduler): T = {
      val future: CancelableFuture[T] = task.runAsync(s)
      future.onComplete {
        case Success(_)         ⇒ ()
        case Failure(exception) ⇒ println(Console.RED + s"TASK ERROR: $exception")
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
    Path(config.getString("fluence.node.storage.rocksDb.dataDir")).deleteRecursively()
    LoggerConfig.level = LogLevel.OFF
  }

}

