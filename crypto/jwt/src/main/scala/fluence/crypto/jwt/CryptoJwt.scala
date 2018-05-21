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

package fluence.crypto.jwt

import cats.syntax.compose._
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.codec.bits.BitsCodecs
import fluence.codec.bits.BitsCodecs._
import fluence.codec.{CodecError, PureCodec}
import fluence.codec.circe.CirceCodecs
import fluence.crypto.{Crypto, CryptoError, KeyPair}
import fluence.crypto.signature.SignAlgo.CheckerFn
import fluence.crypto.signature.{PubKeyAndSignature, SignAlgo, Signature, Signer}
import io.circe.{Decoder, Encoder}
import scodec.bits.{Bases, ByteVector}

/**
 * Primitivized version of JWT.
 *
 * @param readPubKey Gets public key from decoded Header and Claim
 * @tparam H Header type
 * @tparam C Claim type
 */
class CryptoJwt[H: Encoder: Decoder, C: Encoder: Decoder](
  readPubKey: PureCodec.Func[(H, C), KeyPair.Public]
) {
  // Public Key reader, with errors lifted to Crypto
  private val readPK = Crypto.fromOtherFunc(readPubKey)(Crypto.liftCodecErrorToCrypto)

  private val headerAndClaimCodec = Crypto.codec(CryptoJwt.headerClaimCodec[H, C])

  private val signatureCodec = Crypto.codec(CryptoJwt.signatureCodec)

  private val stringTripleCodec = Crypto.codec(CryptoJwt.stringTripleCodec)

  // Take a JWT string, parse and deserialize it, check signature, return Header and Claim on success
  def reader(checkerFn: CheckerFn): Crypto.Func[String, (H, C)] =
    Crypto.liftFuncPoint[String, (H, C)](
      jwtToken ⇒
        for {
          triple ← stringTripleCodec.inverse.pointAt(jwtToken)
          (hc @ (h, c), s) = triple
          headerAndClaim ← headerAndClaimCodec.inverse.pointAt(hc)
          pk ← readPK.pointAt(headerAndClaim)
          signature ← signatureCodec.inverse.pointAt(s)
          plainData = ByteVector((h + c).getBytes())
          _ ← SignAlgo.checkerFunc(checkerFn).pointAt(PubKeyAndSignature(pk, signature) → plainData)
        } yield headerAndClaim
    )

  // With the given Signer, serialize Header and Claim into JWT string, signing it on the way
  def writer(signer: Signer): Crypto.Func[(H, C), String] =
    Crypto.liftFuncPoint[(H, C), String](
      headerAndClaim ⇒
        for {
          pk ← readPK.pointAt(headerAndClaim)
          _ ← Crypto
            .liftFuncEither[Boolean, Unit](
              Either.cond(_, (), CryptoError("JWT encoded PublicKey doesn't match with signer's PublicKey"))
            )
            .pointAt(pk == signer.publicKey)
          hc ← headerAndClaimCodec.direct.pointAt(headerAndClaim)
          plainData = ByteVector((hc._1 + hc._2).getBytes())
          signature ← signer.sign.pointAt(plainData)
          s ← signatureCodec.direct.pointAt(signature)
          jwtToken ← stringTripleCodec.direct.pointAt((hc, s))
        } yield jwtToken
    )
}

object CryptoJwt {

  private val alphabet = Bases.Alphabets.Base64Url

  private val strVec = BitsCodecs.base64AlphabetToVector(alphabet).swap

  val signatureCodec: PureCodec[Signature, String] =
    PureCodec.liftB[Signature, ByteVector](_.sign, Signature(_)) andThen strVec

  private def jsonCodec[T: Encoder: Decoder]: PureCodec[T, String] =
    CirceCodecs.circeJsonCodec[T] andThen
      CirceCodecs.circeJsonParseCodec andThen
      PureCodec.liftB[String, Array[Byte]](_.getBytes(), new String(_)) andThen
      PureCodec[Array[Byte], ByteVector] andThen
      strVec

  val stringTripleCodec: PureCodec[((String, String), String), String] =
    PureCodec.liftEitherB(
      {
        case ((a, b), c) ⇒ Right(s"$a.$b.$c")
      },
      s ⇒
        s.split('.').toList match {
          case a :: b :: c :: Nil ⇒ Right(((a, b), c))
          case l ⇒ Left(CodecError("Wrong number of dot-divided parts, expected: 3, actual: " + l.size))
      }
    )

  def headerClaimCodec[H: Encoder: Decoder, C: Encoder: Decoder]: PureCodec[(H, C), (String, String)] =
    jsonCodec[H] split jsonCodec[C]
}
