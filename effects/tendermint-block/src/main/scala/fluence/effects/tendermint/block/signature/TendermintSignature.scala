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

package fluence.effects.tendermint.block.signature

import fluence.crypto.KeyPair
import fluence.crypto.ecdsa.Ecdsa
import fluence.effects.tendermint.block.protobuf.Protobuf
import proto3.tendermint._
import scodec.bits.ByteVector

/**
 * Implementation of Tendermint's Ed25519 signature
 */
private[block] object TendermintSignature {

  /**
   * Verifies Ed25519 signature for specified message and pubKey, using BouncyCastle library
   *
   * @param message Signed message
   * @param pubKey Public key for the signature
   * @param signature Signatore of the message
   * @return True if signature is correct, false otherwise
   */
  private def verifyBC(message: Array[Byte], pubKey: Array[Byte], signature: Array[Byte]): Boolean = {
    import org.bouncycastle.crypto.params.Ed25519PublicKeyParameters
    import org.bouncycastle.crypto.signers.Ed25519Signer

    val publicKey = new Ed25519PublicKeyParameters(pubKey, 0)
    val signer = new Ed25519Signer
    signer.init(false, publicKey)
    signer.update(message, 0, message.length)
    signer.verifySignature(signature)
  }

  /**
   * Verifies that signatures in Vote are correct
   *
   * Signatures are verified against protobuf-encoded canonical Vote
   *
   * @param vote Vote to verify
   * @param chainID could be taken from e.g., InitChain
   * @param pubKey Public key to check signature against
   * @return True if signature is correct, false otherwise
   */
  def verifyVote(vote: Vote, chainID: String, pubKey: Array[Byte]): Boolean = {
    val canonicalVote = Canonical.vote(vote, chainID)
    val bytes = Protobuf.encodeLengthPrefixed(canonicalVote)
    verifyBC(bytes, pubKey, vote.signature.toByteArray)
  }
}
