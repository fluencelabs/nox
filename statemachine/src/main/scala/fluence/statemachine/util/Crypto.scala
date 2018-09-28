/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package fluence.statemachine.util

import java.util.Base64

import fluence.crypto.CryptoError
import fluence.statemachine.PublicKey
import fluence.statemachine.tree.MerkleHash
import net.i2p.crypto.eddsa.spec.{EdDSANamedCurveTable, EdDSAPublicKeySpec}
import net.i2p.crypto.eddsa.{EdDSAEngine, EdDSAPublicKey}
import org.bouncycastle.jcajce.provider.digest.SHA3
import scodec.bits.ByteVector
import java.security.MessageDigest

import scala.util.Try

object Crypto {

  private val unsafeHasher: fluence.crypto.Crypto.Hasher[Array[Byte], Array[Byte]] =
    fluence.crypto.Crypto.liftFuncEither(
      bytes ⇒
        Try {
          val digest = MessageDigest.getInstance("SHA-256")
          digest.digest(bytes)
        }.toEither.left
          .map(err ⇒ CryptoError(s"Unexpected error when hashing by SHA-256.", Some(err)))
    )

  /**
   * Verifies suggested `signature` of given `data` against known EdDSA-25519 `publicKey`.
   *
   * @param signature suggested signature (in Base64)
   * @param data signed data (in UTF-8 plaintext)
   * @param publicKey EdDSA-25519 public key (in Base64)
   * @return whether verification was successful
   */
  def verify(signature: String, data: String, publicKey: PublicKey): Boolean = {
    val verificationPassed = for {
      signatureBytes <- Try(Base64.getDecoder.decode(signature)).toEither
      dataBytes <- Try(data.getBytes("UTF-8")).toEither
      hashed <- Try(unsafeHasher.unsafe(dataBytes)).toEither
      publicKeyBytes <- Try(Base64.getDecoder.decode(publicKey)).toEither

      edParams <- Option(EdDSANamedCurveTable.getByName(EdDSANamedCurveTable.ED_25519)).toRight("Curve spec not found")
      keySpec <- Try(new EdDSAPublicKeySpec(publicKeyBytes, edParams)).toEither

      key = new EdDSAPublicKey(keySpec)
      engine = new EdDSAEngine()
      _ <- Try(engine.initVerify(key)).toEither
    } yield engine.verifyOneShot(hashed, signatureBytes)
    verificationPassed.getOrElse(false)
  }

  /**
   * Computes SHA3-256 digest for given data.
   *
   * @param data text data
   */
  def sha3Digest256(data: String): MerkleHash = sha3Digest256(data.getBytes("UTF-8"))

  /**
   * Computes SHA3-256 digest for given data.
   *
   * @param data binary data
   */
  def sha3Digest256(data: Array[Byte]): MerkleHash = MerkleHash(ByteVector(new SHA3.Digest256().digest(data)))
}
