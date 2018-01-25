package fluence.crypto.signature

import fluence.crypto.keypair.KeyPair
import scodec.bits.ByteVector

case class Signature(publicKey: KeyPair.Public, sign: ByteVector)
