package fluence.crypto.signature

import java.nio.ByteBuffer

import fluence.crypto.keypair.KeyPair

trait Signer {
  def publicKey: KeyPair.Public

  def sign(plain: Array[Byte]): Signature
}

object Signer {
  class DumbSigner(keyPair: KeyPair) extends Signer {
    override def publicKey: KeyPair.Public = keyPair.publicKey

    override def sign(plain: Array[Byte]): Signature =
      Signature(keyPair.publicKey, ByteBuffer.wrap(plain.reverse))
  }
}
