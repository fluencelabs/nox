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

package fluence.client

import cats.MonadError
import fluence.btree.client.MerkleBTreeClient.ClientState
import fluence.crypto.SignAlgo
import fluence.crypto.algorithm.Ecdsa
import fluence.crypto.cipher.{ Crypt, NoOpCrypt }
import fluence.crypto.hash.CryptoHasher
import fluence.crypto.keypair.KeyPair
import fluence.dataset.BasicContract
import fluence.dataset.client.{ ClientDatasetStorage, Contracts }
import fluence.dataset.protocol.storage.DatasetStorageRpc
import fluence.kad.Kademlia
import fluence.kad.protocol.{ Contact, Key }
import monix.eval.Task
import scodec.bits.ByteVector

import scala.collection.concurrent.TrieMap
import scala.language.higherKinds

class FluenceClient(
    kademlia: Kademlia[Task, Contact],
    contracts: Contracts[Task, BasicContract, Contact],
    signAlgo: SignAlgo,
    storageRpc: Contact ⇒ DatasetStorageRpc[Task],
    storageHasher: CryptoHasher[Array[Byte], Array[Byte]]
)(implicit ME: MonadError[Task, Throwable]) {

  private val datasetCache = TrieMap.empty[KeyPair.Public, Task[ClientDatasetStorage[String, String]]]

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
    loadDatasetFromCache(ac.kp.publicKey, restoreDataset(ac))
  }

  /**
   * Create string dataset with noop-encryption
   * private until multiple datasets per authorized client is supported
   * @param contact node where we will store dataset
   * @param clientState merkle root if it is not a new dataset
   * @return dataset representation
   */
  private def addNonEncryptedDataset(ac: AuthorizedClient, contact: Contact, clientState: Option[ClientState]): Task[ClientDatasetStorage[String, String]] = {
    addDataset(ac, storageRpc(contact), NoOpCrypt.forString, NoOpCrypt.forString, clientState, storageHasher)
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

  private def loadDatasetFromCache(pk: KeyPair.Public, dataStorage: Task[ClientDatasetStorage[String, String]]): Task[ClientDatasetStorage[String, String]] = {
    datasetCache.getOrElseUpdate(pk, dataStorage.memoizeOnSuccess)
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
    signAlgo: SignAlgo = Ecdsa.signAlgo,
    storageHasher: CryptoHasher[Array[Byte], Array[Byte]]): FluenceClient = {
    new FluenceClient(kademliaClient, contracts, signAlgo, storageRpc, storageHasher)
  }
}
