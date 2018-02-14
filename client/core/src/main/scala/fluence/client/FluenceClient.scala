package fluence.client

import cats.kernel.Monoid
import fluence.btree.client.MerkleBTreeClient.ClientState
import fluence.crypto.SignAlgo
import fluence.crypto.cipher.Crypt
import fluence.crypto.hash.JdkCryptoHasher
import fluence.crypto.keypair.KeyPair
import fluence.dataset.BasicContract
import fluence.dataset.client.{ClientDatasetStorage, Contracts}
import fluence.dataset.protocol.storage.DatasetStorageRpc
import fluence.kad.{Kademlia, KademliaConf, KademliaMVar}
import fluence.kad.protocol.{Contact, KademliaRpc, Key, Node}
import monix.eval.{MVar, Task}
import scodec.bits.ByteVector

case class Nonce(number: Int) {
  def next = Nonce(number + 1)

  def toKey(pk: KeyPair.Public) = Key(number.toByte +: pk.value)
}

class FluenceClient(kademliaMVar: Kademlia[Task, Contact], contracts: Contracts[Task, BasicContract, Contact], signAlgo: SignAlgo) {

  val authorizedCache = MVar(Map.empty[KeyPair.Public, AuthorizedClient])
  val contractsCache = MVar(Map.empty[KeyPair.Public, List[BasicContract]])

  /**
    *
    * @param nonce some numbers
    * @param storageRpc transport, do it implicit maybe?
    * @param keyCrypt key and value crypt is depends on keyPair?
    * @param valueCrypt key and value crypt is depends on keyPair?
    * @param clientState we need a possibility to restore client state
    * @tparam K maybe string for simplification for the first time
    * @tparam V maybe string for simplification for the first time
    * @return ready-to-ride dataset
    */
  def addDataset[K, V](ac: AuthorizedClient, nonce: ByteVector = ByteVector.empty,
                       storageRpc: DatasetStorageRpc[Task],
                       keyCrypt: Crypt[Task, K, Array[Byte]],
                       valueCrypt: Crypt[Task, V, Array[Byte]],
                       clientState: Option[ClientState] = None
                      ): ClientDatasetStorage[K, V] = {
    val datasetId = nonce ++ ac.kp.publicKey.value

    //should we have possible to choose different algorithms?
    val hasher = JdkCryptoHasher.Sha256

    //we need a possibility to restore client state
    val clientState = None

    ClientDatasetStorage(datasetId.toArray, hasher, storageRpc, keyCrypt, valueCrypt, clientState)
  }

  def restoreContracts(pk: KeyPair.Public): Task[Map[ByteVector, BasicContract]] = {
    def findRec(nonce: ByteVector, listOfContracts: Map[ByteVector, BasicContract]) = {
      Task.tailRecM((nonce, listOfContracts)) { case (n, l) =>
        contracts.find(Key(n ++ pk.value))
          .map(bc => Left((n, l.updated(n, bc))))
          .onErrorHandleWith {
            case Contracts.NotFound => Task.pure(Right(l))
            case e => Task.raiseError(e)
          }
      }
    }

    findRec(ByteVector("01".getBytes()), Map.empty)
  }

  def loadKeyPair(kp: KeyPair): Task[AuthorizedClient] = {
    for {
      cache <- authorizedCache.take
      (ac, newCache) = cache.get(kp.publicKey) match {
        case Some(auth) => (auth, cache)
        case None =>
          val auth = AuthorizedClient(kp)
          (auth, cache.updated(kp.publicKey, auth))
      }
      _ <- authorizedCache.put(newCache)
    } yield ac
  }

  def generatePair(): Task[AuthorizedClient] = {
    (for {
      kp <- signAlgo.generateKeyPair[Task]()
      ac <- loadKeyPair(kp)
    } yield ac).collectRight
  }

}

object FluenceClient {
  def apply(nodeId: Key, contact: Task[Contact],
               client: Contact ⇒ KademliaRpc[Task, Contact],
               conf: KademliaConf,
               checkNode: Node[Contact] ⇒ Task[Boolean], contracts: Contracts[Task, BasicContract, Contact], signAlgo: SignAlgo): FluenceClient = {
    val kademliaClient = KademliaMVar(Monoid.empty[Key], contact, client, conf, checkNode)
    new FluenceClient(kademliaClient, contracts, signAlgo)
  }
}
