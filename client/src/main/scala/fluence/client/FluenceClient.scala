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

import cats.data.OptionT
import cats.~>
import fluence.btree.client.MerkleBTreeClient.ClientState
import fluence.client.core.ClientReplicationWrapper
import fluence.contract.BasicContract
import fluence.contract.client.Contracts
import fluence.crypto.SignAlgo
import fluence.crypto.algorithm.Ecdsa
import fluence.crypto.cipher.Crypt
import fluence.crypto.hash.CryptoHasher
import fluence.crypto.keypair.KeyPair
import fluence.dataset.client.{ ClientDatasetStorage, ClientDatasetStorageApi }
import fluence.dataset.protocol.DatasetStorageRpc
import fluence.kad.Kademlia
import fluence.kad.protocol.{ Contact, Key }
import monix.eval.Task
import monix.execution.Scheduler.Implicits.global
import scodec.bits.ByteVector

import scala.concurrent.Future
import scala.language.higherKinds

class FluenceClient(
    kademlia: Kademlia[Task, Contact],
    contracts: Contracts[Task, BasicContract, Contact],
    signAlgo: SignAlgo,
    storageRpc: Contact ⇒ DatasetStorageRpc[Task],
    storageHasher: CryptoHasher[Array[Byte], Array[Byte]]
) extends slogging.LazyLogging {

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
   * Restore dataset from cache or from contract in kademlia net
   *
   * @param keyPair    Key pair of client
   * @param keyCrypt   Encryption method for key
   * @param valueCrypt Encryption method for value
   * @return Dataset or None if no contract in kademlia network
   */
  def getDataset(
    keyPair: KeyPair,
    keyCrypt: Crypt[Task, String, Array[Byte]],
    valueCrypt: Crypt[Task, String, Array[Byte]]
  ): Task[Option[ClientDatasetStorageApi[Task, String, String]]] = {
    loadDatasetFromCache(keyPair.publicKey, restoreReplicatedDataset(keyPair, keyCrypt, valueCrypt)) // todo: do replication or don't, should be configurable
  }

  /**
   * Create string dataset with noop-encryption
   * private until multiple datasets per authorized client is supported
   *
   * @param contact     node where we will store dataset
   * @param clientState merkle root if it is not a new dataset
   * @return dataset representation
   */
  private def addEncryptedDataset(
    keyPair: KeyPair,
    contact: Contact,
    clientState: Option[ClientState],
    keyCrypt: Crypt[Task, String, Array[Byte]],
    valueCrypt: Crypt[Task, String, Array[Byte]]
  ): Task[ClientDatasetStorage[String, String]] =
    addDataset(
      keyPair,
      storageRpc(contact),
      keyCrypt,
      valueCrypt,
      clientState,
      storageHasher
    )

  /**
   *
   * Use string only info now
   * private until multiple datasets per authorized client is supported
   *
   * @param storageRpc  transport to dataset
   * @param clientState merkle root of dataset, stored in contract
   * @param nonce       some numbers to iterate through the list of data sets, empty-only for now
   * @return dataset representation
   */
  private def addDataset(
    keyPair: KeyPair,
    storageRpc: DatasetStorageRpc[Task],
    keyCrypt: Crypt[Task, String, Array[Byte]],
    valueCrypt: Crypt[Task, String, Array[Byte]],
    clientState: Option[ClientState],
    hasher: CryptoHasher[Array[Byte], Array[Byte]],
    nonce: ByteVector = ByteVector.empty
  ): Task[ClientDatasetStorage[String, String]] = {

    for {
      datasetId ← Key.sha1[Task]((nonce ++ keyPair.publicKey.value).toArray)
    } yield ClientDatasetStorage(datasetId.value.toArray, hasher, storageRpc, keyCrypt, valueCrypt, clientState)
  }

  private def loadDatasetFromCache(
    pk: KeyPair.Public,
    dataStorage: Task[Option[ClientDatasetStorageApi[Task, String, String]]]
  ): Task[Option[ClientDatasetStorageApi[Task, String, String]]] = {
    dataStorage.memoizeOnSuccess
  }

  def createNewContract(
    keyPair: KeyPair,
    participantsRequired: Int,
    keyCrypt: Crypt[Task, String, Array[Byte]],
    valueCrypt: Crypt[Task, String, Array[Byte]]
  ): Task[ClientReplicationWrapper[String, String]] = {
    import fluence.contract.ops.ContractWrite._
    for {
      key ← Key.fromKeyPair[Task](keyPair)
      signer = signAlgo.signer(keyPair)
      offer ← BasicContract.offer[Task](key, participantsRequired = participantsRequired, signer = signer)
      newContract ← contracts.allocate(offer, dc ⇒ WriteOps[Task, BasicContract](dc).sealParticipants(signer))
      _ = logger.debug("New allocated contract: " + newContract)
      nodes ← findContactsOfAllParticipants(newContract)
      datasets ← Task.sequence(
        nodes.map(contact ⇒
          addEncryptedDataset(keyPair, contact, Some(ClientState(newContract.executionState.merkleRoot)), keyCrypt, valueCrypt)
            .map(store ⇒ store → contact)
        )
      )
    } yield new ClientReplicationWrapper(datasets.toList)
  }

  /**
   * Try to find contract in kademlia net and restore dataset from contract
   */
  // multi-write support
  private def restoreReplicatedDataset(
    keyPair: KeyPair,
    keyCrypt: Crypt[Task, String, Array[Byte]],
    valueCrypt: Crypt[Task, String, Array[Byte]]
  ): Task[Option[ClientReplicationWrapper[String, String]]] = {
    for {
      key ← Key.fromKeyPair[Task](keyPair)
      bcOp ← contracts.find(key).attempt.map(_.toOption)
      dataStorages ← bcOp match {

        case Some(basicContract) ⇒ //create storage from existed contract
          logger.debug(s"Client found contract in kademlia net: $basicContract")
          for {
            nodes ← findContactsOfAllParticipants(basicContract)
            datasets ← Task.sequence(
              nodes.map(contact ⇒
                addEncryptedDataset(keyPair, contact, Some(ClientState(basicContract.executionState.merkleRoot)), keyCrypt, valueCrypt)
                  .map(store ⇒ store → contact)
              )
            )
          } yield Some(datasets)
        case None ⇒ Task.pure(None)
      }
    } yield dataStorages.map(ds ⇒ new ClientReplicationWrapper(ds.toList))
  }

  /** Finds contacts of all contract participants, or return exception otherwise */
  private def findContactsOfAllParticipants(basicContract: BasicContract): Task[Seq[Contact]] = {
    Task.gather(
      basicContract
        .participants
        .map {
          case (key, _) ⇒
            OptionT(kademlia.findNode(key, 3))
              .map(node ⇒ node.contact)
              .getOrElseF(Task.raiseError(new IllegalArgumentException(s"Participant contract isn't found for key=$key")))
        }.toSeq
    )
  }
}

object FluenceClient extends slogging.LazyLogging {

  private implicit def runTask[F[_]]: Task ~> Future = new (Task ~> Future) {
    override def apply[A](fa: Task[A]): Future[A] = fa.runAsync
  }

  private implicit def runFuture[F[_]]: Future ~> Task = new (Future ~> Task) {
    override def apply[A](fa: Future[A]): Task[A] = Task.fromFuture(fa)
  }

  private implicit def runId[F[_]]: F ~> F = new (F ~> F) {
    override def apply[A](fa: F[A]): F[A] = fa
  }

  /**
   * Client for multiple authorized users (with different key pairs).
   * Only one dataset for one user is supported at this moment.
   *
   * @param kademliaClient client for kademlia network, could be shared, it is necessary to join with seed node before
   * @param contracts      client for work with contracts, could be shared
   * @param storageRpc     rpc for dataset communication
   * @param signAlgo       main algorithm for key verification
   */
  def apply(
    kademliaClient: Kademlia[Task, Contact],
    contracts: Contracts[Task, BasicContract, Contact],
    storageRpc: Contact ⇒ DatasetStorageRpc[Task],
    signAlgo: SignAlgo = Ecdsa.signAlgo,
    storageHasher: CryptoHasher[Array[Byte], Array[Byte]]
  ): FluenceClient = {
    new FluenceClient(kademliaClient, contracts, signAlgo, storageRpc, storageHasher)
  }
}
