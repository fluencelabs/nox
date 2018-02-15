package fluence.client

import cats.instances.try_._
import fluence.btree.client.MerkleBTreeClient.ClientState
import fluence.crypto.cipher.{ Crypt, NoOpCrypt }
import fluence.crypto.hash.JdkCryptoHasher
import fluence.crypto.keypair.KeyPair
import fluence.dataset.client.ClientDatasetStorage
import fluence.dataset.protocol.storage.DatasetStorageRpc
import fluence.kad.protocol.Key
import monix.eval.Task
import scodec.bits.ByteVector

import scala.util.Try

case class AuthorizedClient(kp: KeyPair) {

  def addDefaultDataset(storageRpc: DatasetStorageRpc[Task], clientState: Option[ClientState]) = {
    addDataset(storageRpc, NoOpCrypt.forString, NoOpCrypt.forString, clientState)
  }

  /**
   *
   * Use string only info now
   * nonce some numbers to iterate through the list of data sets, empty-only for now
   * @param storageRpc transport, do it implicit maybe?
   * @param keyCrypt key and value crypt is depends on keyPair?
   * @param valueCrypt key and value crypt is depends on keyPair?
   * @param clientState we need a possibility to restore client state
   * @return ready-to-ride dataset
   */
  def addDataset(
    storageRpc: DatasetStorageRpc[Task],
    keyCrypt: Crypt[Task, String, Array[Byte]],
    valueCrypt: Crypt[Task, String, Array[Byte]],
    clientState: Option[ClientState]
  ): ClientDatasetStorage[String, String] = {
    val nonce: ByteVector = ByteVector.empty

    val datasetId = Key.sha1[Try]((nonce ++ kp.publicKey.value).toArray).get

    //should we have possible to choose different algorithms?
    val hasher = JdkCryptoHasher.Sha256

    ClientDatasetStorage(datasetId.value.toArray, hasher, storageRpc, keyCrypt, valueCrypt, clientState)
  }
}
