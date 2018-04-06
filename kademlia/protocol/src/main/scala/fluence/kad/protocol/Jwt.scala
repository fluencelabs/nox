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

package fluence.kad.protocol

import cats.Monad
import cats.data.EitherT
import cats.syntax.compose._
import fluence.codec.PureCodec
import fluence.crypto.SignAlgo.CheckerFn
import fluence.crypto.algorithm.CryptoErr
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.{Signature, Signer}
import io.circe._
import io.circe.parser._
import scodec.bits.{Bases, ByteVector}
import fluence.codec.bits.BitsCodecs._
import fluence.codec.circe.CirceCodecs._

import scala.language.higherKinds

/**
 * Primitivized implementation for JWT, used in [[Contact]] (de)serialization.
 */
private[protocol] object Jwt {
  private val alphabet = Bases.Alphabets.Base64Url

  private val bytesString = PureCodec.liftB[String, Array[Byte]](_.getBytes(), new String(_))

  private val base64StringBytes =
    base64AlphabetToVector(alphabet) andThen PureCodec[ByteVector, Array[Byte]] andThen bytesString.swap

  private val base64json = base64StringBytes andThen PureCodec[String, Json]

  class WritePartial[F[_]: Monad] {

    /**
     *
     * @param header JWT header object
     * @param claim  JWT claim (data) object
     * @param signer Signer
     * @tparam H Header type
     * @tparam C Claim type
     * @return
     */
    def apply[H: Encoder, C: Encoder](header: H, claim: C, signer: Signer): EitherT[F, CryptoErr, String] = {
      val h = ByteVector(Encoder[H].apply(header).noSpaces.getBytes()).toBase64(alphabet)
      val c = ByteVector(Encoder[C].apply(claim).noSpaces.getBytes()).toBase64(alphabet)

      signer
        .sign[F](ByteVector((h + c).getBytes))
        .map(_.sign.toBase64(alphabet))
        .map(h + "." + c + "." + _)
    }
  }

  /**
   * Takes header and claim objects along with theirs Circe Json encoders and signer, and serializes to JWT token string.
   * Notice that either header or claim must contain corresponding PublicKey.
   *
   * @tparam F Effect for signer.sign
   * @return Serialized JWT
   */
  def write[F[_]: Monad]: WritePartial[F] = new WritePartial[F]

  /**
   * Parses JWT header and claim from string representation and checks the signature.
   * You must verify that PublicKey is correct for the sender.
   *
   * @param token JWT token generated with [[write]]
   * @param getPk Getter for primary key from header and claim
   * @param checkerFn Creates checker for specified public key
   * @tparam F Effect for signature checker
   * @tparam H Header type
   * @tparam C Claim type
   * @return Deserialized header and claim, or error
   */
  def read[F[_]: Monad, H: Decoder, C: Decoder](
    token: String,
    getPk: (H, C) ⇒ Either[NoSuchElementException, KeyPair.Public]
  )(implicit checkerFn: CheckerFn): EitherT[F, Throwable, (H, C)] = // InputErr :+: CryptoErr :+: CNil
    token.split('.').toList match {
      case h :: c :: s :: Nil ⇒
        for {
          hbv ← EitherT.fromOption(
            ByteVector.fromBase64(h, alphabet),
            new IllegalArgumentException("Can't read base64 header, got " + h)
          )
          cbv ← EitherT.fromOption(
            ByteVector.fromBase64(c, alphabet),
            new IllegalArgumentException("Can't read base64 claim, got " + c)
          )
          sgn ← EitherT.fromOption(
            ByteVector.fromBase64(s, alphabet),
            new IllegalArgumentException("Can't read base64 signature, got " + s)
          )

          hc ← EitherT.fromEither[F](for {
            hj ← parse(new String(hbv.toArray))
            cj ← parse(new String(cbv.toArray))
            header ← Decoder[H].decodeJson(hj)
            claim ← Decoder[C].decodeJson(cj)
          } yield (header, claim))

          pk ← EitherT.fromEither(getPk(hc._1, hc._2))

          _ ← checkerFn(pk)
            .check(Signature(sgn), ByteVector((h + c).getBytes()))
            .leftMap(err ⇒ err: Throwable)

        } yield hc

      case _ ⇒
        EitherT.leftT(new IllegalArgumentException("Token malformed: must contain exactly two dots"))

    }

}
