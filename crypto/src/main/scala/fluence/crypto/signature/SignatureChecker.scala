package fluence.crypto.signature

import scodec.bits.ByteVector

trait SignatureChecker {
  def check(signature: Signature, plain: ByteVector): Boolean
}

object SignatureChecker {
  case object DumbChecker extends SignatureChecker {
    override def check(signature: Signature, plain: ByteVector): Boolean =
      signature.sign == plain.reverse
  }
}
