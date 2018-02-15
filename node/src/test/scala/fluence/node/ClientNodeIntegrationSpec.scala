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

import java.net.InetAddress

import cats.data.Ior
import cats.kernel.Monoid
import cats.{ Monad, ~> }
import com.typesafe.config.{ ConfigFactory, ConfigValueFactory }
import fluence.btree.client.MerkleBTreeClient.ClientState
import fluence.client.{ ClientComposer, FluenceClient }
import fluence.crypto.SignAlgo
import fluence.crypto.algorithm.Ecdsa
import fluence.crypto.cipher.NoOpCrypt
import fluence.crypto.hash.TestCryptoHasher
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

import scala.concurrent.Future
import scala.concurrent.duration._
import scala.language.higherKinds
import scala.reflect.io.Path
import scala.util.{ Failure, Success }

class ClientNodeIntegrationSpec extends WordSpec with Matchers with ScalaFutures with BeforeAndAfterAll {
  override implicit def patienceConfig: PatienceConfig = PatienceConfig(Span(3, Seconds), Span(250, Milliseconds))

  private val algo: SignAlgo = new SignAlgo(Ecdsa.ecdsa_secp256k1_sha256)

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

  "Node" should {
    "get an exception" when {
      "port already used" ignore {

      }

      // todo finish fail test cases

    }

    "joins the Kademlia network" in {

      runNodes { servers ⇒
        val firstContact = servers.head._1
        val secondContact = servers.last._1

        servers.foreach {
          case (_, s) ⇒
            s.services.flatMap(_.kademlia.join(Seq(firstContact, secondContact), 8)).taskValue
        }
      }

    }

    // todo finish success test cases

  }

  "Client" should {
    "get an exception" when {
      "asks non existing node" in {
        val client = ClientComposer.grpc[Task](GrpcClient.builder)
        val dummyContact = Contact(
          InetAddress.getByName("localhost"), 80, KeyPair.Public(ByteVector("k".getBytes)), 0L, "gitHash", Ior.right("sign")
        )
        val kadClient = client.service[KademliaClient[Task]](dummyContact)
        val result = kadClient.ping().failed.taskValue

        result shouldBe a[StatusRuntimeException]
        // todo there should be more describable exception appears like NetworkException or TimeoutException with cause

      }
      "reads non existing contract" in {
        val client = ClientComposer.grpc[Task](GrpcClient.builder)

        runNodes { servers ⇒
          val firstContact: Contact = servers.head._1
          val contractsApi = createContractApi(firstContact, client)
          val resultContact = contractsApi.find(Key.fromString[Task]("non-exists contract").taskValue).failed.taskValue

          resultContact shouldBe NotFound
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

      // todo finish fail test cases
    }

    "success allocate a contract" in {
      import fluence.dataset.contract.ContractRead._
      import fluence.dataset.contract.ContractWrite._

      val client = ClientComposer.grpc[Task](GrpcClient.builder)

      runNodes { servers ⇒

        val seedContact: Contact = makeKadNetwork(servers)

        val contractsApi = createContractApi(seedContact, client)

        val keyPair = algo.generateKeyPair[Task]().value.taskValue.right.get
        val kadKey = Key.fromKeyPair[Task](keyPair).taskValue
        val signer = algo.signer(keyPair)

        val offer = BasicContract.offer[Task](kadKey, participantsRequired = 4, signer = signer).taskValue
        offer.checkOfferSeal[Task](algo.checker).taskValue shouldBe true

        println("offer participants === " + offer.participants)

        // TODO: add test with wrong signature or other errors
        val accepted = contractsApi.allocate(offer, c ⇒ WriteOps[Task, BasicContract](c).sealParticipants(signer)).taskValue

        accepted.participants.size shouldBe 4

        contractsApi.find(kadKey).taskValue shouldBe accepted

      }
    }

    "create dataset" in {
      println("create client")
      val client = ClientComposer.grpc[Task](GrpcClient.builder)

      runNodes { servers ⇒

        val seedContact: Contact = makeKadNetwork(servers)

        val conf = KademliaConf(100, 1, 5, 5.seconds)
        val kademliaRpc = client.service[KademliaClient[Task]] _

        val emptyKey = Monoid.empty[Key]
        val check = TransportSecurity.canBeSaved[Task](emptyKey, acceptLocal = true)
        println("create kademlia mvar")
        val kademliaClient = KademliaMVar(emptyKey, Task.never, kademliaRpc, conf, check)
        val storageRpc = client.service[DatasetStorageRpc[Task]] _

        println("create client")
        val flClient = FluenceClient.apply(kademliaClient, createContractApi(seedContact, client), algo, storageRpc)

        println("join seeds")
        flClient.joinSeeds(List(seedContact)).taskValue()
        println("seed joined")

        println("gen kp")
        val ac = flClient.generatePair().taskValue()

        println("dataset creation")
        val ds = flClient.getOrCreateDataset(ac).taskValue()
        println(ds)
        println("dataset created")
        println("puting value")

        ds.put("jey", "esrk").taskValue()
        println("value puted")
        println(ds.get("jey").taskValue())
      }
    }

    // todo finish success test cases

  }

  private def createContractApi[T <: HList](seedContact: Contact, client: GrpcClient[T], servers: Map[Contact, NodeComposer] = Map.empty)(implicit
    s1: ops.hlist.Selector[T, ContractsCacheRpc[Task, BasicContract]],
    s2: ops.hlist.Selector[T, ContractAllocatorRpc[Task, BasicContract]],
    s3: ops.hlist.Selector[T, KademliaClient[Task]]
  ) = {

    val conf = KademliaConf(100, 10, 2, 5.seconds)
    val clKey = Monoid.empty[Key]
    val check = TransportSecurity.canBeSaved[Task](clKey, acceptLocal = true)
    val kademliaRpc = client.service[KademliaClient[Task]] _
    val kademliaClient = KademliaMVar(clKey, Task.never, kademliaRpc, conf, check)

    val kadCl = if (servers.nonEmpty) servers.head._2.services.map(_.kademlia).taskValue
    else kademliaClient

    kadCl.join(Seq(seedContact), 2).taskValue()

    new Contracts[Task, BasicContract, Contact](
      maxFindRequests = 10,
      maxAllocateRequests = _ ⇒ 20,
      checker = algo.checker,
      kademlia = kadCl,
      cacheRpc = contact ⇒ client.service[ContractsCacheRpc[Task, BasicContract]](contact),
      allocatorRpc = contact ⇒ client.service[ContractAllocatorRpc[Task, BasicContract]](contact)
    )
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
      TestCryptoHasher,
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
        contractsCacheStore
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

  override protected def afterAll(): Unit = {
    Path(config.getString("fluence.node.storage.rocksDb.dataDir")).deleteRecursively()
  }

}

