package fluence.crypto.algorithm

import fluence.crypto.keypair.KeyPair
import scodec.bits.ByteVector

trait CipherFunctions extends Algorithm {
  def encrypt(keyPair: KeyPair, message: ByteVector): ByteVector
  def decrypt(keyPair: KeyPair, message: ByteVector): ByteVector
}