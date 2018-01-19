package fluence.crypto.keypair

import java.nio.ByteBuffer

// TODO: generate keypairs
case class KeyPair(publicKey: KeyPair.Public, secretKey: KeyPair.Secret)

object KeyPair {
  case class Public(value: ByteBuffer) extends AnyVal
  case class Secret(value: ByteBuffer) extends AnyVal

  def fromBytes(pk: Array[Byte], sk: Array[Byte]): KeyPair = KeyPair(Public(ByteBuffer.wrap(pk)), Secret(ByteBuffer.wrap(sk)))
}
