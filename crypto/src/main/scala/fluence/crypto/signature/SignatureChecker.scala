package fluence.crypto.signature

import fluence.crypto.algorithm.Ecdsa
import scodec.bits.ByteVector

trait SignatureChecker {
  def check(signature: Signature, plain: ByteVector): Boolean
}

object SignatureChecker {
  case object DumbChecker extends SignatureChecker {
    override def check(signature: Signature, plain: ByteVector): Boolean =
      signature.sign == plain.reverse
  }

  case object EcdsaChecker extends SignatureChecker {
    override def check(signature: Signature, plain: ByteVector): Boolean =
      Ecdsa.ecdsa_secp256k1_sha256.verify(signature, plain)
  }
}
