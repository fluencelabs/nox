package fluence.crypto.signature

import java.nio.ByteBuffer

trait SignatureChecker {
  def check(signature: Signature, plain: Array[Byte]): Boolean
}

object SignatureChecker {
  case object DumbChecker extends SignatureChecker {
    override def check(signature: Signature, plain: Array[Byte]): Boolean =
      signature.sign == ByteBuffer.wrap(plain.reverse)
  }
}
