package fluence.crypto.keypair

import scodec.bits.ByteVector

// TODO: generate keypairs
case class KeyPair(publicKey: KeyPair.Public, secretKey: KeyPair.Secret)

object KeyPair {
  case class Public(value: ByteVector) extends AnyVal
  case class Secret(value: ByteVector) extends AnyVal

  def fromBytes(pk: Array[Byte], sk: Array[Byte]): KeyPair = KeyPair(Public(ByteVector(pk)), Secret(ByteVector(sk)))
}
