package fluence.client

import cats.kernel.Monoid
import cats.{ MonadError, ~> }
import fluence.btree.client.MerkleBTreeClient.ClientState
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
import monix.execution.Scheduler.Implicits.global

import scala.concurrent.Future
import scala.concurrent.duration._
import scala.language.higherKinds

case class Nonce(number: Int) {
  def next = Nonce(number + 1)

  def toKey(pk: KeyPair.Public): Task[Key] = Key.fromBytes[Task]((number.toByte +: pk.value).toArray)
}

class FluenceClient(
    kademlia: Kademlia[Task, Contact],
    contracts: Contracts[Task, BasicContract, Contact],
    signAlgo: SignAlgo, storageRpc: Contact ⇒ DatasetStorageRpc[Task]
)(implicit ME: MonadError[Task, Throwable]) {

  val authorizedCache = MVar(Map.empty[KeyPair.Public, AuthorizedClient])
  val datasetCache = MVar(Map.empty[KeyPair.Public, ClientDatasetStorage[String, String]])

  //use this when we will have multiple datasets on one authorized user
  def restoreContracts(pk: KeyPair.Public): Task[Map[Key, BasicContract]] = {
    def findRec(nonce: Nonce, listOfContracts: Map[Key, BasicContract]) = {
      Task.tailRecM((nonce, listOfContracts)) {
        case (n, l) ⇒
          val t = for {
            k ← n.toKey(pk)
            bc ← contracts.find(k)
          } yield Left((n, l.updated(k, bc)))
          t.onErrorHandleWith (_ ⇒ Task.pure(Right(l)))
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

  def restoreDataset(ac: AuthorizedClient): Task[ClientDatasetStorage[String, String]] = {
    import fluence.dataset.contract.ContractWrite._
    val signer = signAlgo.signer(ac.kp)
    for {
      key ← Key.fromKeyPair(ac.kp)
      bcOp ← contracts.find(key).attempt.map(_.toOption)
      dataStorage ← bcOp match {
        //create datastorage with old merkle root
        case Some(bc) ⇒
          for {
            //TODO avoid _.head
            ns ← kademlia.lookupIterative(bc.participants.head._1, 3)
            _ = println("NODES === " + ns)
            ds = ac.addDefaultDataset(storageRpc(ns.head.contact), Some(ClientState(bc.executionState.merkleRoot.toArray)))
          } yield ds
        //new datastorage
        case None ⇒
          for {
            //TODO avoid _.head
            offer ← BasicContract.offer(key, participantsRequired = 1, signer = signer)
            _ = println("CONTRACT === " + offer)
            _ = println("CONTRACT PARTICIPANTS === " + offer.participants)
            accepted ← contracts.allocate(offer, dc ⇒ dc.sealParticipants(signer))
            _ = println("CONTRACT ACCEPTED === " + accepted.participants)
            ns ← kademlia.lookupIterative(accepted.participants.head._1, 1)
            _ = println("NODES === " + ns)
            ds = ac.addDefaultDataset(storageRpc(ns.head.contact), None)
          } yield ds
      }
    } yield dataStorage
  }

  def getOrCreateDataset(ac: AuthorizedClient): Task[ClientDatasetStorage[String, String]] = {
    for {
      fromCache ← loadDatasetFromCache(ac.kp.publicKey)
      ds ← fromCache match {
        case Some(d) ⇒ Task.pure(d)
        case None    ⇒ restoreDataset(ac)
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

  def joinSeeds(contacts: List[Contact]): Task[Unit] = {
    kademlia.join(contacts, 1)
  }

}

object FluenceClient {
  def apply(
    kademliaClient: Kademlia[Task, Contact],
    contracts: Contracts[Task, BasicContract, Contact],
    signAlgo: SignAlgo,
    storageRpc: Contact ⇒ DatasetStorageRpc[Task]): FluenceClient = {
    new FluenceClient(kademliaClient, contracts, signAlgo, storageRpc)
  }

  def default(seedContacts: List[Contact]): FluenceClient = {

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

    val key = Monoid.empty[Key]
    val check = TransportSecurity.canBeSaved[Task](key, acceptLocal = true)

    val client = ClientComposer.grpc[Task](GrpcClient.builder)
    val storageRpc: Contact ⇒ DatasetStorageRpc[Task] = client.service[DatasetStorageRpc[Task]]

    val kademliaRpc = client.service[KademliaClient[Task]] _
    val kademliaClient = KademliaMVar(key, Task.never, kademliaRpc, conf, check)

    val contracts = new Contracts[Task, BasicContract, Contact](
      maxFindRequests = 10,
      maxAllocateRequests = _ ⇒ 20,
      checker = signAlgo.checker,
      kademlia = kademliaClient,
      cacheRpc = contact ⇒ client.service[ContractsCacheRpc[Task, BasicContract]](contact),
      allocatorRpc = contact ⇒ client.service[ContractAllocatorRpc[Task, BasicContract]](contact)
    )

    FluenceClient(kademliaClient, contracts, signAlgo, storageRpc)
  }
}
