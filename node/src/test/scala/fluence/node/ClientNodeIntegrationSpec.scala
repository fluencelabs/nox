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
import cats.effect.Effect
import cats.~>
import com.typesafe.config.{ ConfigFactory, ConfigValueFactory }
import fluence.btree.client.MerkleBTreeClient.ClientState
import fluence.client.ClientComposer
import fluence.crypto.SignAlgo
import fluence.crypto.algorithm.Ecdsa
import fluence.crypto.cipher.NoOpCrypt
import fluence.crypto.hash.TestCryptoHasher
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.SignatureChecker
import fluence.dataset.BasicContract
import fluence.dataset.client.{ ClientDatasetStorage, Contracts }
import fluence.dataset.protocol.storage.DatasetStorageRpc
import fluence.kad.grpc.client.KademliaClient
import fluence.kad.protocol.{ Contact, Key }
import fluence.transport.grpc.client.GrpcClient
import io.grpc.StatusRuntimeException
import monix.eval.Task
import monix.eval.instances.CatsEffectForTask
import monix.execution.Scheduler.Implicits.global
import monix.execution.{ CancelableFuture, Scheduler }
import org.rocksdb.RocksDBException
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.time.{ Milliseconds, Seconds, Span }
import org.scalatest.{ BeforeAndAfterAll, Matchers, WordSpec }
import scodec.bits.ByteVector

import scala.concurrent.Future
import scala.concurrent.duration._
import scala.language.higherKinds
import scala.reflect.io.Path
import scala.util.{ Failure, Success }

class ClientNodeIntegrationSpec extends WordSpec with Matchers with ScalaFutures with BeforeAndAfterAll {
  override implicit def patienceConfig: PatienceConfig = PatienceConfig(Span(5, Seconds), Span(250, Milliseconds))

  private val algo: SignAlgo = new SignAlgo(Ecdsa.ecdsa_secp256k1_sha256)

  private implicit val checker: SignatureChecker = algo.checker

  private implicit val taskEffect: Effect[Task] = new CatsEffectForTask()

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

      // todo finish fail test cases

    }

    "joins the Kademlia network" ignore { // todo enable when rockDb problem will be solved

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
        val newClient = ClientComposer.grpc[Task](GrpcClient.builder)
        val dummyContact = Contact(
          InetAddress.getByName("localhost"), 80, KeyPair.Public(ByteVector("k".getBytes)), 0L, "gitHash", Ior.right("sign")
        )
        val kadClient = newClient.service[KademliaClient[Task]](dummyContact)
        val result = kadClient.ping().failed.taskValue

        result shouldBe a[StatusRuntimeException]
        // todo there should be more describable exception appears like NetworkException or TimeoutException with cause

      }
      "reads non existing contract" ignore { // todo enable when rockDb problem will be solved
        val newClient = ClientComposer.grpc[Task](GrpcClient.builder)

        runNodes { servers ⇒
          val firstContact: Contact = servers.head._1
          val contractsApi = createContactApi(servers)
          val resultContact = contractsApi.find(Key.fromString[Task]("non-exists contract").taskValue).failed.taskValue

          resultContact shouldBe a[StatusRuntimeException]
          // todo transmit error from server: there should be Contracts.NotFound exception instead of StatusRuntimeException
          // resultContact shouldBe NotFound
        }

      }

      "reads and writes from dataset without contracting" ignore { // todo enable when rockDb problem will be solved
        val newClient = ClientComposer.grpc[Task](GrpcClient.builder)

        runNodes { servers ⇒
          val firstContact: Contact = servers.head._1
          val storageRpc = newClient.service[DatasetStorageRpc[Task]](firstContact)
          val datasetStorage = createDatasetStorage("dummy dataset".getBytes, storageRpc)

          val getResponse = datasetStorage.get("request key").failed.taskValue
          getResponse shouldBe a[RocksDBException]
          // todo transmit error from server: there should be DatasetNotFound exception (or something else) instead of RocksDBException

          val putResponse = datasetStorage.put("key", "value").failed.taskValue
          // todo transmit error from server: there should be DatasetNotFound exception (or something else) instead of RocksDBException
          putResponse shouldBe a[RocksDBException]

        }
      }

      // todo finish fail test cases
    }

    "success allocate a contract" in {
      import fluence.dataset.contract.ContractRead._
      import fluence.dataset.contract.ContractWrite._

      val newClient = ClientComposer.grpc[Task](GrpcClient.builder)

      runNodes { servers ⇒

        val seedContact: Contact = makeKadNetwork(servers)

        val contractsApi = createContactApi(servers)

        val keyPair = algo.generateKeyPair[Task]().value.taskValue.right.get
        val kadKey = Key.fromKeyPair[Task](keyPair).taskValue
        val signer = algo.signer(keyPair)

        val offer = BasicContract.offer(kadKey, participantsRequired = 4, signer = signer).taskValue
        offer.checkOfferSeal(algo.checker).taskValue shouldBe true

        // TODO: add test with wrong signature or other errors
        val accepted = contractsApi.allocate(offer, _.sealParticipants(signer)).taskValue

        accepted.participants.size shouldBe 4

        contractsApi.find(kadKey).taskValue shouldBe accepted

      }
    }

    // todo finish success test cases

  }

  private def createContactApi(servers: Map[Contact, NodeComposer]) = {
    new Contracts[Task, BasicContract, Contact](
      maxFindRequests = 10,
      maxAllocateRequests = _ ⇒ 20,
      checker = algo.checker,
      kademlia = servers.head._2.services.map(_.kademlia).taskValue, // TODO IMPORTANT: use client-side Kademlia
      cacheRpc = contact ⇒ servers.find { case (c, _) ⇒ contact.publicKey.value == c.publicKey.value }.get._2
        .services.map(_.contractsCache).taskValue,
      allocatorRpc = contact ⇒
        servers.find{ case (c, _) ⇒ contact.publicKey.value == c.publicKey.value }.get._2
          .services.map(_.contractAllocator).taskValue
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

    val servers: Map[Contact, NodeComposer] = (0 to numberOfNodes).map { n ⇒
      val port = 3100 + n
      val composer = new NodeComposer(
        algo.generateKeyPair[Task]().value.taskValue.right.get,
        algo,
        config
          .withValue("fluence.transport.grpc.server.localPort", ConfigValueFactory.fromAnyRef(port))
          .withValue("fluence.transport.grpc.server.externalPort", ConfigValueFactory.fromAnyRef(null))
          .withValue("fluence.transport.grpc.server.acceptLocal", ConfigValueFactory.fromAnyRef(true)),
        "node_cache_" + n
      )

      composer.server.flatMap(_.contact).taskValue → composer
    }.toMap

    try {
      servers.foreach(_._2.server.flatMap(_.start()).runAsync.futureValue)
      action(servers)
    } finally {
      // shutting down all nodes
      servers.foreach(_._2.server.foreach(_.shutdown(3.second)))
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

