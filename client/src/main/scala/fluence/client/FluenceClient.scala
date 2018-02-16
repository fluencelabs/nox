package fluence.client

import cats.MonadError
import fluence.btree.client.MerkleBTreeClient.ClientState
import fluence.crypto.SignAlgo
import fluence.crypto.algorithm.Ecdsa
import fluence.crypto.cipher.{ Crypt, NoOpCrypt }
import fluence.crypto.hash.{ CryptoHasher, JdkCryptoHasher }
import fluence.crypto.keypair.KeyPair
import fluence.dataset.BasicContract
import fluence.dataset.client.{ ClientDatasetStorage, Contracts }
import fluence.dataset.protocol.storage.DatasetStorageRpc
import fluence.kad.{ Kademlia, MVarMapCache }
import fluence.kad.protocol.{ Contact, Key }
import monix.eval.Task
import scodec.bits.ByteVector

import scala.language.higherKinds
import scala.util.Try

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

  /**
   * Create dataset for authorized client, or restore it, if it is in the contract in kademlia net
   */
  def getOrCreateDataset(ac: AuthorizedClient): Task[ClientDatasetStorage[String, String]] = {
    loadDatasetFromCache(ac.kp.publicKey, restoreDataset(ac)).map(_.get)
  }

  /**
   * Generate new key pair for new user and store it in cache
   * @return new authorized user
   */
  def generatePair(): Task[AuthorizedClient] = {
    for {
      kp ← signAlgo.generateKeyPair[Task]().value.flatMap(ME.fromEither)
      ac ← loadOrCreateAuthorizedClient(kp)
    } yield ac
  }

  /**
   * Create authorized client with existing key pair or restore it from cache
   */
  def loadOrCreateAuthorizedClient(kp: KeyPair): Task[AuthorizedClient] = {
    authorizedCache.getOrAdd(kp.publicKey, Some(AuthorizedClient(kp))).map(_.get)
  }

  /**
   * Create string dataset with noop-encryption
   * private until multiple datasets per authorized client is supported
   * @param contact node where we will store dataset
   * @param clientState merkle root if it is not a new dataset
   * @return dataset representation
   */
  private def addNonEncryptedDataset(ac: AuthorizedClient, contact: Contact, clientState: Option[ClientState]): Task[ClientDatasetStorage[String, String]] = {
    addDataset(ac, storageRpc(contact), NoOpCrypt.forString, NoOpCrypt.forString, clientState, JdkCryptoHasher.Sha256)
  }

  /**
   *
   * Use string only info now
   * private until multiple datasets per authorized client is supported
   * @param storageRpc transport to dataset
   * @param keyCrypt algorithm to encrypt keys
   * @param valueCrypt algorithm to encrypt values
   * @param clientState merkle root of dataset, stored in contract
   * @param nonce some numbers to iterate through the list of data sets, empty-only for now
   * @return dataset representation
   */
  private def addDataset(
    ac: AuthorizedClient,
    storageRpc: DatasetStorageRpc[Task],
    keyCrypt: Crypt[Task, String, Array[Byte]],
    valueCrypt: Crypt[Task, String, Array[Byte]],
    clientState: Option[ClientState],
    hasher: CryptoHasher[Array[Byte], Array[Byte]],
    nonce: ByteVector = ByteVector.empty
  ): Task[ClientDatasetStorage[String, String]] = {

    for {
      datasetId ← Key.sha1((nonce ++ ac.kp.publicKey.value).toArray)
    } yield ClientDatasetStorage(datasetId.value.toArray, hasher, storageRpc, keyCrypt, valueCrypt, clientState)
  }

  private def loadDatasetFromCache(pk: KeyPair.Public, dataStorage: Task[ClientDatasetStorage[String, String]]): Task[Option[ClientDatasetStorage[String, String]]] = {
    datasetCache.getOrAddF(pk, dataStorage.map(Option.apply))
  }

  /**
   * try to find dataset in cache,
   * if it is absent, try to find contract in net and restore dataset from contract
   * if it is no contract, create new contract and dataset
   *
   */
  private def restoreDataset(ac: AuthorizedClient): Task[ClientDatasetStorage[String, String]] = {
    import fluence.dataset.contract.ContractWrite._
    val signer = signAlgo.signer(ac.kp)
    for {
      key ← Key.fromKeyPair(ac.kp)
      bcOp ← contracts.find(key).attempt.map(_.toOption)
      dataStorage ← bcOp match {
        //create datastorage with old merkle root
        case Some(bc) ⇒
          for {
            //TODO avoid _.head and get, magic numbers
            nodeOp ← kademlia.findNode(bc.participants.head._1, 3)
            ds ← addNonEncryptedDataset(ac, nodeOp.get.contact, Some(ClientState(bc.executionState.merkleRoot.toArray)))
          } yield ds
        //new datastorage
        case None ⇒
          for {
            //TODO avoid _.head, magic numbers, add errors
            offer ← BasicContract.offer(key, participantsRequired = 1, signer = signer)
            accepted ← contracts.allocate(offer, dc ⇒ dc.sealParticipants(signer))
            nodeOp ← kademlia.findNode(accepted.participants.head._1, 10)
            ds ← addNonEncryptedDataset(ac, nodeOp.get.contact, None)
          } yield ds
      }
    } yield dataStorage
  }
}

object FluenceClient {
  /**
   * Client for multiple authorized users (with different key pairs).
   * Only one dataset for one user is supported at this moment.
   * @param kademliaClient client for kademlia network, could be shared, it is necessary to join with seed node before
   * @param contracts client for work with contracts, could be shared
   * @param storageRpc rpc for dataset communication
   * @param signAlgo main algorithm for key verification
   */
  def apply(
    kademliaClient: Kademlia[Task, Contact],
    contracts: Contracts[Task, BasicContract, Contact],
    storageRpc: Contact ⇒ DatasetStorageRpc[Task],
    signAlgo: SignAlgo = Ecdsa.signAlgo): FluenceClient = {
    new FluenceClient(kademliaClient, contracts, signAlgo, storageRpc)
  }
}
