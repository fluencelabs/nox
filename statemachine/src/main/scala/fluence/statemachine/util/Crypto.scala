/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

package fluence.statemachine.util

import java.util.Base64

import fluence.statemachine.PublicKey
import fluence.statemachine.tree.MerkleHash
import net.i2p.crypto.eddsa.spec.{EdDSANamedCurveTable, EdDSAPublicKeySpec}
import net.i2p.crypto.eddsa.{EdDSAEngine, EdDSAPublicKey}
import org.bouncycastle.jcajce.provider.digest.SHA3
import scodec.bits.ByteVector

import scala.util.Try

object Crypto {

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
      publicKeyBytes <- Try(Base64.getDecoder.decode(publicKey)).toEither

      edParams <- Option(EdDSANamedCurveTable.getByName(EdDSANamedCurveTable.ED_25519)).toRight("Curve spec not found")
      keySpec <- Try(new EdDSAPublicKeySpec(publicKeyBytes, edParams)).toEither

      key = new EdDSAPublicKey(keySpec)
      engine = new EdDSAEngine()
      _ <- Try(engine.initVerify(key)).toEither
    } yield engine.verifyOneShot(dataBytes, signatureBytes)
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
