package fluence.crypto.signature

import fluence.crypto.keypair.KeyPair
import scodec.bits.ByteVector

trait Signer {
  def publicKey: KeyPair.Public

  def sign(plain: ByteVector): Signature
}

object Signer {
  class DumbSigner(keyPair: KeyPair) extends Signer {
    override def publicKey: KeyPair.Public = keyPair.publicKey

    override def sign(plain: ByteVector): Signature =
      Signature(keyPair.publicKey, plain.reverse)
  }
}
