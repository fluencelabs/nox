package fluence.swarm.crypto
import fluence.crypto.{Crypto, CryptoError}
import org.web3j.crypto.Hash
import scodec.bits.ByteVector

import scala.util.Try

object Keccak256Hasher {

  /**
   * Default hash function in Swarm. Arrow from plain bytes to hash bytes.
   */
  val hasher: Crypto.Hasher[ByteVector, ByteVector] =
    Crypto.liftFuncEither(
      bytes ⇒
        Try {
          ByteVector(Hash.sha3(bytes.toArray))
        }.toEither.left
          .map(err ⇒ CryptoError(s"Unexpected error when hashing by SHA-3.", Some(err)))
    )
}
