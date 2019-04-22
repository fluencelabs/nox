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

package fluence.effects.tendermint.block

import fluence.crypto.KeyPair
import fluence.crypto.ecdsa.Ecdsa
import proto3.tendermint._
import scodec.bits.ByteVector

// Ed25519
object Signature {

  def verifyBC(message: Array[Byte], pubKey: Array[Byte], signature: Array[Byte]): Boolean = {
    import org.bouncycastle.crypto.params.Ed25519PublicKeyParameters
    import org.bouncycastle.crypto.signers.Ed25519Signer

    val publicKey = new Ed25519PublicKeyParameters(pubKey, 0)
    val signer = new Ed25519Signer
    signer.init(false, publicKey)
    signer.update(message, 0, message.length)
    signer.verifySignature(signature)
  }

  // Doesn't work :( That's because it's Curve25519, not Ed25519, and their keys arent compatible
  // Fluence Crypto uses ECNamedCurveTable, and it seems it doesn't support Ed25519
  def verifyFluenceCrypto(message: Array[Byte], pubKey: Array[Byte], signature: Array[Byte]): Boolean = {
    println(s"Signature.verify key ${ByteVector(pubKey).toHex}")
    val ed25519 = new Ecdsa("Curve25519", "NONEwithECDSA", None)
    val result = ed25519
      .verify(
        KeyPair.Public(ByteVector(pubKey)),
        fluence.crypto.signature.Signature(signature),
        ByteVector(message)
      )
    result.value.left.foreach(e => println(s"Signature.verify error: ${e.getMessage()}"))
    result.isRight
  }

  // ChainID could be taken from e.g., InitChain
  def verifyVote(vote: Vote, chainID: String, pubKey: Array[Byte]): Boolean = {
    val canonicalVote = canonicalize(vote, chainID)
    val bytes = Amino.encodeLengthPrefixed(canonicalVote)
    verifyBC(bytes, pubKey, vote.signature.toByteArray)
  }

  def canonicalize(header: PartSetHeader): CanonicalPartSetHeader = {
    CanonicalPartSetHeader(header.hash, header.total)
  }

  def canonicalize(blockID: BlockID): CanonicalBlockID = {
    CanonicalBlockID(blockID.hash, blockID.parts.map(canonicalize))
  }

  def canonicalize(vote: Vote, chainID: String): CanonicalVote = {
    val blockID = vote.blockId.map(canonicalize)
    CanonicalVote(vote.`type`, vote.height, vote.round, blockID, vote.timestamp, chainID)
  }
}
