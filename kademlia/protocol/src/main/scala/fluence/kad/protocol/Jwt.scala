package fluence.kad.protocol

import cats.Monad
import cats.data.EitherT
import fluence.crypto.algorithm.CryptoErr
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.{ Signature, SignatureChecker, Signer }
import io.circe._
import io.circe.parser._
import scodec.bits.{ Bases, ByteVector }

import scala.language.higherKinds

/**
 * Primitivized implementation for JWT, used in [[Contact]] (de)serialization.
 */
private[protocol] object Jwt {
  private val alphabet = Bases.Alphabets.Base64Url

  class WritePartial[F[_] : Monad] {
    /**
     *
     * @param header JWT header object
     * @param claim  JWT claim (data) object
     * @param signer Signer
     * @tparam H Header type
     * @tparam C Claim type
     * @return
     */
    def apply[H : Encoder, C : Encoder](header: H, claim: C, signer: Signer): EitherT[F, CryptoErr, String] = {
      val h = ByteVector(Encoder[H].apply(header).noSpaces.getBytes()).toBase64(alphabet)
      val c = ByteVector(Encoder[C].apply(claim).noSpaces.getBytes()).toBase64(alphabet)

      signer.sign[F](ByteVector((h + c).getBytes))
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
  def write[F[_] : Monad]: WritePartial[F] = new WritePartial[F]

  /**
   * Parses JWT header and claim from string representation and checks the signature.
   * You must verify that PublicKey is correct for the sender.
   *
   * @param token   JWT token generated with [[write]]
   * @param getPk   Getter for primary key from header and claim
   * @param checker Signature checker
   * @tparam F Effect for signature checker
   * @tparam H Header type
   * @tparam C Claim type
   * @return Deserialized header and claim, or error
   */
  def read[F[_] : Monad, H : Decoder, C : Decoder](
    token: String,
    getPk: (H, C) ⇒ Either[NoSuchElementException, KeyPair.Public],
    checker: SignatureChecker): EitherT[F, Throwable, (H, C)] =
    token.split('.').toList match {
      case h :: c :: s :: Nil ⇒

        for {
          hbv ← EitherT.fromOption(ByteVector.fromBase64(h, alphabet), new IllegalArgumentException("Can't read base64 header"))
          cbv ← EitherT.fromOption(ByteVector.fromBase64(c, alphabet), new IllegalArgumentException("Can't read base64 claim"))

          hc ← EitherT.fromEither[F](for {
            hj ← parse(new String(hbv.toArray))
            cj ← parse(new String(cbv.toArray))
            header ← Decoder[H].decodeJson(hj)
            claim ← Decoder[C].decodeJson(cj)
          } yield (header, claim))

          pk ← EitherT.fromEither(getPk(hc._1, hc._2))

          _ ← checker.check(Signature(pk, ByteVector(s.getBytes)), ByteVector((h + c).getBytes())).leftMap(err ⇒ err: Throwable) // TODO: coproduct

        } yield hc

      case _ ⇒
        EitherT.leftT(new IllegalArgumentException("Token malformed: must contain exactly two dots")) // TODO: coproduct

    }

}
