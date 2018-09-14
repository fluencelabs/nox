package fluence.swarm.crypto
import fluence.crypto.{Crypto, CryptoError}
import org.web3j.crypto.{ECKeyPair, Sign}
import scodec.bits.ByteVector

import scala.util.Try

/**
 * Default signing logic adapted from the BitcoinJ ECKey.
 *
 * @see https://github.com/bitcoinj/bitcoinj/blob/master/core/src/main/java/org/bitcoinj/core/ECKey.java
 */
private[swarm] object Secp256k1Signer {

  type Signer[A, B] = Crypto.Func[A, B]

  import fluence.swarm.helpers.SignatureDataOps._

  /**
   * Arrow from plain bytes to signed bytes.
   *
   * @param kp elliptic Curve SECP-256k1 generated key pair
   */
  def signer(kp: ECKeyPair): Signer[ByteVector, ByteVector] =
    Crypto.liftFuncEither(
      bytes ⇒
        Try {
          val signData = Sign.signMessage(bytes.toArray, kp, false)
          ByteVector(signData.toByteArray)
        }.toEither.left.map(err ⇒ CryptoError(s"Unexpected error when signing by ECDSA algorithm.", Some(err)))
    )
}
