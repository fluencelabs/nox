package fluence.client

import cats.instances.try_._
import fluence.btree.client.MerkleBTreeClient.ClientState
import fluence.crypto.cipher.{Crypt, NoOpCrypt}
import fluence.crypto.hash.{CryptoHasher, JdkCryptoHasher}
import fluence.crypto.keypair.KeyPair
import fluence.dataset.client.ClientDatasetStorage
import fluence.dataset.protocol.storage.DatasetStorageRpc
import fluence.kad.protocol.Key
import monix.eval.Task
import scodec.bits.ByteVector

import scala.util.Try

case class AuthorizedClient(kp: KeyPair) {

  def addDefaultDataset(storageRpc: DatasetStorageRpc[Task], clientState: Option[ClientState]) = {
    addDataset(storageRpc, NoOpCrypt.forString, NoOpCrypt.forString, clientState, JdkCryptoHasher.Sha256)
  }

  /**
   *
   * Use string only info now
   * nonce some numbers to iterate through the list of data sets, empty-only for now
   * @param storageRpc transport to dataset
   * @param keyCrypt algorithm to encrypt keys
   * @param valueCrypt algorithm to encrypt values
   * @param clientState merkle root of dataset, stored in contract
   * @return ready-to-ride dataset
   */
  def addDataset(
    storageRpc: DatasetStorageRpc[Task],
    keyCrypt: Crypt[Task, String, Array[Byte]],
    valueCrypt: Crypt[Task, String, Array[Byte]],
    clientState: Option[ClientState],
    hasher: CryptoHasher[Array[Byte], Array[Byte]],
    nonce: ByteVector = ByteVector.empty
  ): ClientDatasetStorage[String, String] = {

    val datasetId = Key.sha1[Try]((nonce ++ kp.publicKey.value).toArray).get

    ClientDatasetStorage(datasetId.value.toArray, hasher, storageRpc, keyCrypt, valueCrypt, clientState)
  }
}
