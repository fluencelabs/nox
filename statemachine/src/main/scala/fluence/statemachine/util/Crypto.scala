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

import cats.instances.try_._
import fluence.crypto.hash.CryptoHashers
import fluence.statemachine.PublicKey
import fluence.statemachine.tree.MerkleHash
import net.i2p.crypto.eddsa.spec.{EdDSANamedCurveSpec, EdDSANamedCurveTable, EdDSAPrivateKeySpec, EdDSAPublicKeySpec}
import net.i2p.crypto.eddsa.{EdDSAEngine, EdDSAPrivateKey, EdDSAPublicKey}
import org.bouncycastle.jcajce.provider.digest.SHA3
import scodec.bits.ByteVector

import scala.util.{Failure, Success, Try}

object Crypto extends slogging.LazyLogging {

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
      signatureBytes <- Try(Base64.getDecoder.decode(signature))
      dataBytes <- Try(data.getBytes("UTF-8"))
      hashed <- CryptoHashers.Sha256.runF[Try](dataBytes)
      publicKeyBytes <- Try(Base64.getDecoder.decode(publicKey))

      edParams <- Option(EdDSANamedCurveTable.getByName(EdDSANamedCurveTable.ED_25519))
        .fold[Try[EdDSANamedCurveSpec]](Failure(new Exception()))(Success(_))
      keySpec <- Try(new EdDSAPublicKeySpec(publicKeyBytes, edParams))

      publicKey = new EdDSAPublicKey(keySpec)
      engine = new EdDSAEngine()
      _ <- Try(engine.initVerify(publicKey))
      verified <- Try(engine.verifyOneShot(hashed, signatureBytes))
    } yield verified
    verificationPassed.failed.foreach { e =>
      logger.error("An error on verifying signature: {}", e.getMessage)
    }
    verificationPassed.getOrElse(false)
  }

  /**
   * Signs `data` using a given EdDSA-25519 `privateKeyBase64`.
   *
   * @param data text input
   * @return text Base-64 signature
   */
  def sign(data: String, privateKeyBase64: String): String = {
    val signatureTry = for {
      dataBytes <- Try(data.getBytes("UTF-8"))
      hashed <- CryptoHashers.Sha256.runF[Try](dataBytes)
      privateKeyBytes <- Try(Base64.getDecoder.decode(privateKeyBase64))

      edParams <- Option(EdDSANamedCurveTable.getByName(EdDSANamedCurveTable.ED_25519))
        .fold[Try[EdDSANamedCurveSpec]](Failure(new Exception()))(Success(_))
      keySpec <- Try(new EdDSAPrivateKeySpec(privateKeyBytes, edParams))

      privateKey = new EdDSAPrivateKey(keySpec)
      engine = new EdDSAEngine()
      _ <- Try(engine.initSign(privateKey))

      signatureBytes <- Try(engine.signOneShot(hashed))
      signatureString <- Try(Base64.getEncoder.encodeToString(signatureBytes))
    } yield signatureString
    signatureTry.failed.foreach { //
      e =>
        logger.error("An error on obtaining signature: {}", e.getMessage)
    }
    signatureTry.getOrElse("cannot_sign_data")
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
