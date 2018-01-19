package fluence.crypto.signature

import java.nio.ByteBuffer

import fluence.crypto.keypair.KeyPair

case class Signature(publicKey: KeyPair.Public, sign: ByteBuffer)
