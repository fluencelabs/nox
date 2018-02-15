package fluence.client

import java.net.InetAddress

import cats.{ MonadError, ~> }
import cats.data.Ior
import cats.kernel.Monoid
import fluence.crypto.SignAlgo
import fluence.crypto.algorithm.Ecdsa
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.SignatureChecker
import fluence.dataset.BasicContract
import fluence.dataset.client.{ ClientDatasetStorage, Contracts }
import fluence.dataset.protocol.storage.DatasetStorageRpc
import fluence.dataset.protocol.{ ContractAllocatorRpc, ContractsCacheRpc }
import fluence.kad.grpc.client.KademliaClient
import fluence.kad.protocol.{ Contact, Key }
import fluence.kad.{ Kademlia, KademliaConf, KademliaMVar }
import fluence.transport.TransportSecurity
import fluence.transport.grpc.client.GrpcClient
import monix.eval.{ MVar, Task }
import scodec.bits.ByteVector
import monix.execution.Scheduler.Implicits.global

import scala.concurrent.Future
import scala.concurrent.duration._
import scala.language.higherKinds

case class Nonce(number: Int) {
  def next = Nonce(number + 1)

  def toKey(pk: KeyPair.Public) = Key(number.toByte +: pk.value)
}

class FluenceClient(
    kademliaMVar: Kademlia[Task, Contact],
    contracts: Contracts[Task, BasicContract, Contact],
    signAlgo: SignAlgo
)(implicit ME: MonadError[Task, Throwable]) {

  val authorizedCache = MVar(Map.empty[KeyPair.Public, AuthorizedClient])
  val datasetCache = MVar(Map.empty[KeyPair.Public, ClientDatasetStorage[String, String]])

  //use this when we will have multiple datasets on one authorized user
  def restoreContracts(pk: KeyPair.Public): Task[Map[Key, BasicContract]] = {
    def findRec(nonce: Nonce, listOfContracts: Map[Key, BasicContract]) = {
      Task.tailRecM((nonce, listOfContracts)) {
        case (n, l) ⇒
          contracts.find(n.toKey(pk))
            .map(bc ⇒ Left((n, l.updated(n.toKey(pk), bc))))
            .onErrorHandleWith (_ ⇒ Task.pure(Right(l)))
      }
    }
    findRec(Nonce(0), Map.empty)
  }

  def loadDatasetFromCache(pk: KeyPair.Public): Task[Option[ClientDatasetStorage[String, String]]] = {
    for {
      cache ← datasetCache.take
      ds = cache.get(pk)
      _ ← datasetCache.put(cache)
    } yield ds
  }

  def restoreDataset(ac: AuthorizedClient, storageRpc: DatasetStorageRpc[Task]): Task[ClientDatasetStorage[String, String]] = {
    contracts.find(Key(ac.kp.publicKey.value)).attempt.map(_.toOption).map {
      case Some(bc) ⇒ ac.addDefaultDataset(storageRpc) //create datastorage with old merkle root
      case None     ⇒ ac.addDefaultDataset(storageRpc)
    }

  }

  def getOrCreateDataset(ac: AuthorizedClient, storageRpc: DatasetStorageRpc[Task]): Task[ClientDatasetStorage[String, String]] = {
    for {
      fromCache ← loadDatasetFromCache(ac.kp.publicKey)
      ds ← fromCache match {
        case Some(d) ⇒ Task.pure(d)
        case None    ⇒ restoreDataset(ac, storageRpc)
      }
    } yield ds
  }

  def loadOrCreateAuthorizedClient(kp: KeyPair): Task[AuthorizedClient] = {
    for {
      cache ← authorizedCache.take
      (ac, newCache) = cache.get(kp.publicKey) match {
        case Some(auth) ⇒ (auth, cache)
        case None ⇒
          val auth = AuthorizedClient(kp)
          (auth, cache.updated(kp.publicKey, auth))
      }
      _ ← authorizedCache.put(newCache)
    } yield ac
  }

  def generatePair(): Task[AuthorizedClient] = {
    for {
      kp ← signAlgo.generateKeyPair[Task]().value.flatMap(ME.fromEither)
      ac ← loadOrCreateAuthorizedClient(kp)
    } yield ac
  }

}

object FluenceClient {
  def apply(conf: KademliaConf): FluenceClient = {

    implicit def runId[F[_]]: F ~> F = new (F ~> F) {
      override def apply[A](fa: F[A]): F[A] = fa
    }

    implicit def runTask[F[_]]: Task ~> Future = new (Task ~> Future) {
      override def apply[A](fa: Task[A]): Future[A] = fa.runAsync
    }

    implicit def runFuture[F[_]]: Future ~> Task = new (Future ~> Task) {
      override def apply[A](fa: Future[A]): Task[A] = Task.fromFuture(fa)
    }

    val conf = KademliaConf(100, 1, 5, 5.seconds)
    val signAlgo = new SignAlgo(Ecdsa.ecdsa_secp256k1_sha256)

    implicit val checker: SignatureChecker = signAlgo.checker

    val contact = Contact(InetAddress.getLocalHost, 5555, KeyPair.Public(ByteVector.empty), 0L, "", Ior.right(""))

    val key = Monoid.empty[Key]
    val check = TransportSecurity.canBeSaved[Task](key, acceptLocal = true)

    val client = ClientComposer.grpc[Task](GrpcClient.builder)

    val kademliaRpc = client.service[KademliaClient[Task]] _
    val kademliaClient = KademliaMVar(key, Task.pure(contact), kademliaRpc, conf, check)

    val contracts = new Contracts[Task, BasicContract, Contact](
      maxFindRequests = 10,
      maxAllocateRequests = _ ⇒ 20,
      checker = signAlgo.checker,
      kademlia = kademliaClient,
      cacheRpc = contact ⇒ client.service[ContractsCacheRpc[Task, BasicContract]](contact),
      allocatorRpc = contact ⇒ client.service[ContractAllocatorRpc[Task, BasicContract]](contact)
    )

    new FluenceClient(kademliaClient, contracts, signAlgo)
  }
}
