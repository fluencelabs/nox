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

package fluence.client.core

import cats.data.OptionT
import cats.effect.IO
import cats.kernel.Monoid
import cats.~>
import fluence.btree.client.MerkleBTreeClient.ClientState
import fluence.contract.BasicContract
import fluence.contract.client.Contracts
import fluence.crypto.{Crypto, KeyPair}
import fluence.crypto.ecdsa.Ecdsa
import fluence.crypto.signature.{SignAlgo, Signer}
import fluence.dataset.client.{ClientDatasetStorage, ClientDatasetStorageApi}
import fluence.dataset.protocol.DatasetStorageRpc
import fluence.kad.protocol.{Contact, ContactSecurity, KademliaRpc, Key}
import fluence.kad.{Kademlia, KademliaConf, KademliaMVar}
import monix.eval.Task
import monix.execution.Scheduler
import monix.reactive.Observable
import scodec.bits.ByteVector

import scala.concurrent.Future
import scala.language.higherKinds

class FluenceClient(
  kademlia: Kademlia[Task, Contact],
  contracts: Contracts[Task, BasicContract],
  signAlgo: SignAlgo,
  storageRpc: Contact ⇒ DatasetStorageRpc[Task, Observable],
  storageHasher: Crypto.Hasher[Array[Byte], Array[Byte]]
) extends slogging.LazyLogging {

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
    keyCrypt: Crypto.Cipher[String],
    valueCrypt: Crypto.Cipher[String]
  ): Task[Option[ClientDatasetStorageApi[Task, Observable, String, String]]] = {
    // todo: do replication or don't, should be configurable
    restoreReplicatedDataset(keyPair, keyCrypt, valueCrypt).memoizeOnSuccess
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
    datasetVer: Long,
    keyCrypt: Crypto.Cipher[String],
    valueCrypt: Crypto.Cipher[String],
    signer: Signer
  ): Task[ClientDatasetStorage[String, String]] =
    addDataset(
      keyPair,
      storageRpc(contact),
      keyCrypt,
      valueCrypt,
      clientState,
      datasetVer,
      storageHasher,
      signer
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
    storageRpc: DatasetStorageRpc[Task, Observable],
    keyCrypt: Crypto.Cipher[String],
    valueCrypt: Crypto.Cipher[String],
    clientState: Option[ClientState],
    datasetVer: Long,
    hasher: Crypto.Hasher[Array[Byte], Array[Byte]],
    signer: Signer,
    nonce: ByteVector = ByteVector.empty
  ): Task[ClientDatasetStorage[String, String]] = {
    for {
      datasetId ← Key.sha1.runF[Task]((nonce ++ keyPair.publicKey.value).toArray)
    } yield
      ClientDatasetStorage(
        datasetId.value.toArray,
        datasetVer,
        hasher,
        storageRpc,
        keyCrypt,
        valueCrypt,
        clientState,
        signer
      )
  }

  def createNewContract(
    keyPair: KeyPair,
    participantsRequired: Int,
    keyCrypt: Crypto.Cipher[String],
    valueCrypt: Crypto.Cipher[String]
  ): Task[ClientDatasetStorageApi[Task, Observable, String, String]] = {
    import fluence.contract.ops.ContractWrite._
    for {
      key ← Key.fromKeyPair.runF[Task](keyPair)
      signer = signAlgo.signer(keyPair)
      offer ← BasicContract.offer[Task](key, participantsRequired = participantsRequired, signer = signer)
      newContract ← contracts
        .allocate(
          offer,
          dc ⇒
            WriteOps[Task, BasicContract](dc)
              .sealParticipants(signer)
              .leftMap(_.message)
        ) // TODO: refactor this to EitherT
        .value
        .flatMap(v ⇒ Task(v.right.get))

      _ = logger.debug("New allocated contract: " + newContract)
      nodes ← findContactsOfAllParticipants(newContract)
      datasets ← Task.sequence(
        nodes.map(
          contact ⇒
            addEncryptedDataset(
              keyPair,
              contact,
              Some(ClientState(newContract.executionState.merkleRoot)),
              newContract.executionState.version,
              keyCrypt,
              valueCrypt,
              signer
            ).map(store ⇒ store → contact)
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
    keyCrypt: Crypto.Cipher[String],
    valueCrypt: Crypto.Cipher[String]
  ): Task[Option[ClientDatasetStorageApi[Task, Observable, String, String]]] = {
    for {
      key ← Key.fromKeyPair.runF[Task](keyPair)
      _ = logger.debug(s"Contract key: $key")
      bcOp ← contracts.find(key).value.attempt
      signer = signAlgo.signer(keyPair)

      dataStorages ← bcOp match {
        case Right(Right(basicContract)) ⇒ //create storage from existed contract
          logger.debug(s"Client found contract in kademlia net: $basicContract")
          for {
            nodes ← findContactsOfAllParticipants(basicContract)
            datasets ← Task.sequence(
              nodes.map(
                contact ⇒
                  addEncryptedDataset(
                    keyPair,
                    contact,
                    Some(ClientState(basicContract.executionState.merkleRoot)),
                    basicContract.executionState.version,
                    keyCrypt,
                    valueCrypt,
                    signer
                  ).map(store ⇒ store → contact)
              )
            )
          } yield Some(datasets)
        case _ ⇒
          logger.warn(s"Contract for specified key=$key was not found")
          Task.pure(None)
      }
    } yield dataStorages.map(ds ⇒ new ClientReplicationWrapper(ds.toList))
  }

  /** Finds contacts of all contract participants, or return exception otherwise */
  private def findContactsOfAllParticipants(basicContract: BasicContract): Task[Seq[Contact]] = {
    Task.gather(
      basicContract.participants.map {
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
    override def apply[A](fa: Task[A]): Future[A] = fa.runAsync(Scheduler.global)
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
    contracts: Contracts[Task, BasicContract],
    storageRpc: Contact ⇒ DatasetStorageRpc[Task, Observable],
    signAlgo: SignAlgo = Ecdsa.signAlgo,
    storageHasher: Crypto.Hasher[Array[Byte], Array[Byte]]
  ): FluenceClient =
    new FluenceClient(kademliaClient, contracts, signAlgo, storageRpc, storageHasher)

  /**
   * Builds FluenceClient with its enclosed services
   *
   * @param seeds Seed nodes to use for network discovery
   * @param signAlgo Signing algorithm
   * @param storageHasher Hasher used for b-tree
   * @param kademliaConf Configuration for Kademlia
   * @param client Access to remote services
   * @return A FluenceClient ready to be used
   */
  def build(
    seeds: Seq[Contact],
    signAlgo: SignAlgo,
    storageHasher: Crypto.Hasher[Array[Byte], Array[Byte]],
    kademliaConf: KademliaConf,
    client: Contact ⇒ ClientServices[Task, BasicContract, Contact]
  ): IO[FluenceClient] = {

    import signAlgo.checker

    logger.info("Creating kademlia client...")
    val kademliaClient = createKademliaClient(kademliaConf, client(_).kademlia)

    logger.info("Connecting to seed node...")

    for {
      _ ← kademliaClient.join(seeds, 4).toIO(Scheduler.global)
      _ = logger.info("Creating contracts api...")
      contracts = Contracts[Task, Task.Par, BasicContract, Contact](
        maxFindRequests = 10,
        maxAllocateRequests = _ ⇒ 20,
        kademlia = kademliaClient,
        cacheRpc = client(_).contractsCache,
        allocatorRpc = client(_).contractAllocator
      )
    } yield FluenceClient(kademliaClient, contracts, client(_).datasetStorage, signAlgo, storageHasher)
  }

  private def createKademliaClient(
    conf: KademliaConf,
    kademliaRpc: Contact ⇒ KademliaRpc[Contact]
  ): Kademlia[Task, Contact] = {
    val check = ContactSecurity.check[IO](Monoid.empty[Key], acceptLocal = true)
    KademliaMVar.client(kademliaRpc, conf, check)
  }
}
