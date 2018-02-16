package fluence.client

import cats.MonadError
import fluence.btree.client.MerkleBTreeClient.ClientState
import fluence.crypto.SignAlgo
import fluence.crypto.keypair.KeyPair
import fluence.dataset.BasicContract
import fluence.dataset.client.{ ClientDatasetStorage, Contracts }
import fluence.dataset.protocol.storage.DatasetStorageRpc
import fluence.kad.{ Kademlia, MVarMapCache }
import fluence.kad.protocol.{ Contact, Key }
import monix.eval.Task

import scala.language.higherKinds

class FluenceClient(
    kademlia: Kademlia[Task, Contact],
    contracts: Contracts[Task, BasicContract, Contact],
    signAlgo: SignAlgo,
    storageRpc: Contact ⇒ DatasetStorageRpc[Task]
)(implicit ME: MonadError[Task, Throwable]) {

  val authorizedCache = new MVarMapCache[KeyPair.Public, Option[AuthorizedClient]](None)
  val datasetCache = new MVarMapCache[KeyPair.Public, Option[ClientDatasetStorage[String, String]]](None)

  //use this when we will have multiple datasets on one authorized user
  /*def restoreContracts(pk: KeyPair.Public): Task[Map[Key, BasicContract]] = {
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
  }*/

  def loadDatasetFromCache(pk: KeyPair.Public, dataStorage: Task[ClientDatasetStorage[String, String]]): Task[Option[ClientDatasetStorage[String, String]]] = {
    datasetCache.getOrAddF(pk, dataStorage.map(Option.apply))
  }

  /**
    * try to find dataset in cache,
    * if it is absent, try to find contract in net and restore dataset from contract
    * if it is no contract, create new contract and dataset
    *
    */
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
            ds = ac.addDefaultDataset(storageRpc(ns.head.contact), Some(ClientState(bc.executionState.merkleRoot.toArray)))
          } yield ds
        //new datastorage
        case None ⇒
          for {
            //TODO avoid _.head
            offer ← BasicContract.offer(key, participantsRequired = 1, signer = signer)
            accepted ← contracts.allocate(offer, dc ⇒ dc.sealParticipants(signer))
            ns ← kademlia.findNode(accepted.participants.head._1, 10)
            ds = ac.addDefaultDataset(storageRpc(ns.head.contact), None)
          } yield ds
      }
    } yield dataStorage
  }

  def getOrCreateDataset(ac: AuthorizedClient): Task[ClientDatasetStorage[String, String]] = {
    loadDatasetFromCache(ac.kp.publicKey, restoreDataset(ac)).map(_.get)
  }

  def loadOrCreateAuthorizedClient(kp: KeyPair): Task[AuthorizedClient] = {
    authorizedCache.getOrAdd(kp.publicKey, Some(AuthorizedClient(kp))).map(_.get)
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
}
