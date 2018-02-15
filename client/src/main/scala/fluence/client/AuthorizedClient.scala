package fluence.client

import fluence.btree.client.MerkleBTreeClient.ClientState
import fluence.crypto.cipher.{ Crypt, NoOpCrypt }
import fluence.crypto.hash.JdkCryptoHasher
import fluence.crypto.keypair.KeyPair
import fluence.dataset.client.ClientDatasetStorage
import fluence.dataset.protocol.storage.DatasetStorageRpc
import monix.eval.Task
import scodec.bits.ByteVector

case class AuthorizedClient(kp: KeyPair) {

  def addDefaultDataset(storageRpc: DatasetStorageRpc[Task]) = {
    addDataset(storageRpc, NoOpCrypt.forString, NoOpCrypt.forString, None)
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
    clientState: Option[ClientState] = None
  ): ClientDatasetStorage[String, String] = {
    val nonce: ByteVector = ByteVector.empty

    val datasetId = nonce ++ kp.publicKey.value

    //should we have possible to choose different algorithms?
    val hasher = JdkCryptoHasher.Sha256

    //we need a possibility to restore client state
    val clientState = None

    val a = ClientDatasetStorage(datasetId.toArray, hasher, storageRpc, keyCrypt, valueCrypt, clientState)

    a
  }
}
