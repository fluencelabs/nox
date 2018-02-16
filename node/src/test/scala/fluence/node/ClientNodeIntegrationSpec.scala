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

import cats.~>
import com.typesafe.config.{ ConfigFactory, ConfigValueFactory }
import fluence.btree.client.MerkleBTreeClient.ClientState
import fluence.client.{ ClientComposer, FluenceClient }
import fluence.crypto.SignAlgo
import fluence.crypto.algorithm.Ecdsa
import fluence.crypto.cipher.NoOpCrypt
import fluence.crypto.hash.JdkCryptoHasher
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.SignatureChecker
import fluence.dataset.BasicContract
import fluence.dataset.client.Contracts.NotFound
import fluence.dataset.client.{ ClientDatasetStorage, Contracts }
import fluence.dataset.protocol.storage.DatasetStorageRpc
import fluence.dataset.protocol.{ ContractAllocatorRpc, ContractsCacheRpc }
import fluence.kad.{ KademliaConf, KademliaMVar }
import fluence.kad.grpc.client.KademliaClient
import fluence.kad.protocol.{ Contact, Key }
import fluence.storage.rocksdb.RocksDbStore
import fluence.transport.TransportSecurity
import fluence.transport.grpc.client.GrpcClient
import io.grpc.StatusRuntimeException
import monix.eval.Task
import monix.execution.Scheduler.Implicits.global
import monix.execution.{ CancelableFuture, Scheduler }
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
  override implicit def patienceConfig: PatienceConfig = PatienceConfig(Span(1, Seconds), Span(100, Milliseconds))

  private val algo: SignAlgo = Ecdsa.signAlgo

  private implicit val checker: SignatureChecker = algo.checker

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
        val server = new ServerSocket(3000)
        try {
          // run node on the busy port
          val port = 3000
          val contractsCacheStore = RocksDbStore[Task]("node_cache", config).taskValue
          val composer = new NodeComposer(
            algo.generateKeyPair[Task]().value.taskValue.right.get,
            algo,
            config
              .withValue("fluence.transport.grpc.server.localPort", ConfigValueFactory.fromAnyRef(port))
              .withValue("fluence.transport.grpc.server.externalPort", ConfigValueFactory.fromAnyRef(null))
              .withValue("fluence.transport.grpc.server.acceptLocal", ConfigValueFactory.fromAnyRef(true)),
            contractsCacheStore
          )
          try {
            val result = composer.server.taskValue.start().failed.taskValue()
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
          val firstContact = servers.head._1
          val exception = servers.head._2.services.taskValue.kademlia.join(Seq(firstContact, dummyContact), 8).failed.taskValue
          exception shouldBe a[RuntimeException]
          exception should have message "Can't join any node among known peers"
        }
      }

    }

    "joins the Kademlia network" in {
      runNodes { servers ⇒
        val firstContact = servers.head._1
        val secondContact = servers.last._1
        servers.foreach {
          case (_, s) ⇒ s.services.flatMap(_.kademlia.join(Seq(firstContact, secondContact), 8)).taskValue
        }
      }
    }

  }

  "Client" should {
    "get an exception" when {

      "asks non existing node" in {
        val client = ClientComposer.grpc[Task](GrpcClient.builder)
        val kadClient = client.service[KademliaClient[Task]](dummyContact)
        val result = kadClient.ping().failed.taskValue

        result shouldBe a[StatusRuntimeException]
        // todo there should be more describable exception appears like NetworkException or TimeoutException with cause

      }

      "reads non existing contract" in {
        val client = ClientComposer.grpc[Task](GrpcClient.builder)
        runNodes { servers ⇒
          val (kademliaClient, contractsApi) = createClientApi(servers.head._1, client)
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
          val (kademliaClient, contractsApi) = createClientApi(seedContact, client)
          val offer = BasicContract.offer[Task](kadKey, participantsRequired = 999, signer = signer).taskValue
          offer.checkOfferSeal[Task](algo.checker).taskValue shouldBe true

          val result = contractsApi.allocate(offer, c ⇒ WriteOps[Task, BasicContract](c).sealParticipants(signer)).failed.taskValue
          result should be equals Contracts.CantFindEnoughNodes(30)
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
          val (kademliaClient, contractsApi) = createClientApi(seedContact, client)
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
        val (kademliaClient, contractsApi) = createClientApi(seedContact, client)

        val acceptedContract = contractsApi.allocate(offer, c ⇒ WriteOps[Task, BasicContract](c).sealParticipants(signer)).taskValue

        acceptedContract.participants.size shouldBe 4
        contractsApi.find(kadKey).taskValue shouldBe acceptedContract
      }
    }

    "success write and read from dataset" in {
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
        val (kademliaClient, contractsApi) = createClientApi(seedContact, client)
        val acceptedContract = contractsApi.allocate(offer, c ⇒ WriteOps[Task, BasicContract](c).sealParticipants(signer)).taskValue

        val (nodeKey, _) = acceptedContract.participants.head
        // we won't find node address by nodeId, cause service FluenceClient is not ready yet, it'll be later
        val nodeContact = servers.find{ case (c, _) ⇒ Key.fromPublicKey[Task](c.publicKey).taskValue == nodeKey }.get._1

        val storageRpc = client.service[DatasetStorageRpc[Task]](nodeContact)
        val datasetStorage = createDatasetStorage(acceptedContract.id.id, storageRpc)

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
    }

    "create dataset" in {
      val client = ClientComposer.grpc[Task](GrpcClient.builder)

      runNodes { servers ⇒

        val seedContact: Contact = makeKadNetwork(servers)

        val storageRpc = client.service[DatasetStorageRpc[Task]] _

        val (kademliaClient, contractApi) = createClientApi(seedContact, client)

        val flClient = FluenceClient.apply(kademliaClient, contractApi, storageRpc)

        val ac = flClient.generatePair().taskValue()

        val ds = flClient.getOrCreateDataset(ac).taskValue()

        ds.put("jey1", "esrk1").taskValue()
        ds.put("jey2", "esrk2").taskValue()
        ds.put("jey3", "esrk3").taskValue()

        //todo this should be equals, but something wrong
        ds.get("jey1").taskValue()
        ds.get("jey2").taskValue()
        ds.get("jey3").taskValue()
      }
    }

    // todo finish success test cases

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
    val kademliaClient = KademliaMVar(clKey, Task.never, kademliaRpc, conf, check)

    kademliaClient.join(Seq(seedContact), 2).taskValue()

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
      JdkCryptoHasher.Sha256,
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
      val port = 3100 + n
      val contractsCacheStore = RocksDbStore[Task]("node_cache_" + n, config).taskValue
      val composer = new NodeComposer(
        algo.generateKeyPair[Task]().value.taskValue.right.get,
        algo,
        config
          .withValue("fluence.transport.grpc.server.localPort", ConfigValueFactory.fromAnyRef(port))
          .withValue("fluence.transport.grpc.server.externalPort", ConfigValueFactory.fromAnyRef(null))
          .withValue("fluence.transport.grpc.server.acceptLocal", ConfigValueFactory.fromAnyRef(true)),
        contractsCacheStore,
        JdkCryptoHasher.Sha256
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

  private implicit class WaitTask[T](task: Task[T]) {
    def taskValue()(implicit s: Scheduler): T = {
      val future: CancelableFuture[T] = task.runAsync(s)
      future.onComplete {
        case Success(_)         ⇒ ()
        case Failure(exception) ⇒ println(Console.RED + s"TASK ERROR: $exception")
      }(s)
      future.futureValue
    }
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

