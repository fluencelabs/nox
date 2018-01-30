package fluence.crypto.algorithm

import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.Signature
import scodec.bits.ByteVector

trait SignatureFunctions extends Algorithm {
  def sign(keyPair: KeyPair, message: ByteVector): Signature
  def verify(signature: Signature, message: ByteVector): Boolean
}
