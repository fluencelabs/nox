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

/**
  * A class that is an authorized user who can use datasets
  * @param kp a pair of keys with a public key that will be used as an address for dataset ids and contracts
  */
case class AuthorizedClient(kp: KeyPair)
