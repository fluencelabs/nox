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

package fluence.crypto.ecdsa

import java.math.BigInteger
import java.security._
import java.security.interfaces.ECPrivateKey

import cats.Monad
import cats.data.EitherT
import fluence.crypto.{KeyPair, _}
import fluence.crypto.hash.JdkCryptoHasher
import fluence.crypto.signature.{SignAlgo, SignatureChecker, Signer}
import org.bouncycastle.jce.ECNamedCurveTable
import org.bouncycastle.jce.interfaces.ECPublicKey
import org.bouncycastle.jce.provider.BouncyCastleProvider
import org.bouncycastle.jce.spec.{ECParameterSpec, ECPrivateKeySpec, ECPublicKeySpec}
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * Elliptic Curve Digital Signature Algorithm
 * @param curveType http://www.bouncycastle.org/wiki/display/JA1/Supported+Curves+%28ECDSA+and+ECGOST%29
 * @param scheme https://bouncycastle.org/specifications.html
 */
class Ecdsa(curveType: String, scheme: String, hasher: Option[Crypto.Hasher[Array[Byte], Array[Byte]]])
    extends JavaAlgorithm {

  import CryptoError.nonFatalHandling
  import Ecdsa._

  val HEXradix = 16

  val generateKeyPair: Crypto.KeyPairGenerator =
    new Crypto.Func[Option[Array[Byte]], KeyPair] {
      override def apply[F[_]](
        input: Option[Array[Byte]]
      )(implicit F: Monad[F]): EitherT[F, CryptoError, fluence.crypto.KeyPair] =
        for {
          ecSpec ← EitherT.fromOption(
            Option(ECNamedCurveTable.getParameterSpec(curveType)),
            CryptoError("Parameter spec for the curve is not available.")
          )
          g ← getKeyPairGenerator
          _ ← nonFatalHandling {
            g.initialize(ecSpec, input.map(new SecureRandom(_)).getOrElse(new SecureRandom()))
          }(s"Could not initialize KeyPairGenerator")
          p ← EitherT.fromOption(Option(g.generateKeyPair()), CryptoError("Could not generate KeyPair. Unexpected."))
          keyPair ← nonFatalHandling {
            //store S number for private key and compressed Q point on curve for public key
            val pk = ByteVector(p.getPublic.asInstanceOf[ECPublicKey].getQ.getEncoded(true))
            val bg = p.getPrivate.asInstanceOf[ECPrivateKey].getS
            val sk = ByteVector.fromValidHex(bg.toString(HEXradix))
            KeyPair.fromByteVectors(pk, sk)
          }("Could not generate KeyPair. Unexpected.")
        } yield keyPair
    }

  def sign[F[_]: Monad](
    keyPair: KeyPair,
    message: ByteVector
  ): EitherT[F, CryptoError, signature.Signature] =
    signMessage(new BigInteger(keyPair.secretKey.value.toHex, HEXradix), message.toArray)
      .map(bb ⇒ fluence.crypto.signature.Signature(ByteVector(bb)))

  def verify[F[_]: Monad](
    publicKey: KeyPair.Public,
    signature: fluence.crypto.signature.Signature,
    message: ByteVector
  ): EitherT[F, CryptoError, Unit] =
    verifySign(publicKey.bytes, signature.bytes, message.toArray)

  private def signMessage[F[_]: Monad](
    privateKey: BigInteger,
    message: Array[Byte]
  ): EitherT[F, CryptoError, Array[Byte]] =
    for {
      ec ← curveSpec
      keySpec ← nonFatalHandling(new ECPrivateKeySpec(privateKey, ec))("Cannot read private key.")
      keyFactory ← getKeyFactory
      signProvider ← getSignatureProvider
      sign ← nonFatalHandling {
        signProvider.initSign(keyFactory.generatePrivate(keySpec))
        signProvider.update(hasher.map(_.unsafe(message)).getOrElse(message))
        signProvider.sign()
      }("Cannot sign message.")

    } yield sign

  private def verifySign[F[_]: Monad](
    publicKey: Array[Byte],
    signature: Array[Byte],
    message: Array[Byte],
  ): EitherT[F, CryptoError, Unit] =
    for {
      ec ← curveSpec
      keySpec ← nonFatalHandling(new ECPublicKeySpec(ec.getCurve.decodePoint(publicKey), ec))("Cannot read public key.")
      keyFactory ← getKeyFactory
      signProvider ← getSignatureProvider
      verify ← nonFatalHandling {
        signProvider.initVerify(keyFactory.generatePublic(keySpec))
        signProvider.update(hasher.map(_.unsafe(message)).getOrElse(message))
        signProvider.verify(signature)
      }("Cannot verify message.")

      _ ← EitherT.cond[F](verify, (), CryptoError("Signature is not verified"))
    } yield ()

  private def curveSpec[F[_]: Monad] =
    nonFatalHandling(ECNamedCurveTable.getParameterSpec(curveType).asInstanceOf[ECParameterSpec])(
      "Cannot get curve parameters."
    )

  private def getKeyPairGenerator[F[_]: Monad] =
    nonFatalHandling(KeyPairGenerator.getInstance(ECDSA, BouncyCastleProvider.PROVIDER_NAME))(
      "Cannot get key pair generator."
    )

  private def getKeyFactory[F[_]: Monad] =
    nonFatalHandling(KeyFactory.getInstance(ECDSA, BouncyCastleProvider.PROVIDER_NAME))(
      "Cannot get key factory instance."
    )

  private def getSignatureProvider[F[_]: Monad] =
    nonFatalHandling(Signature.getInstance(scheme, BouncyCastleProvider.PROVIDER_NAME))(
      "Cannot get signature instance."
    )
}

object Ecdsa {
  //algorithm name in security provider
  val ECDSA = "ECDSA"

  /**
   * size of key is 256 bit
   * `secp256k1` refers to the parameters of the ECDSA curve
   * `NONEwithECDSA with sha-256 hasher` Preferably the size of the key is greater than or equal to the digest algorithm
   * don't use `SHA256WithECDSA` because of non-compatibility with javascript libraries
   */
  val ecdsa_secp256k1_sha256 = new Ecdsa("secp256k1", "NONEwithECDSA", Some(JdkCryptoHasher.Sha256))

  val signAlgo: SignAlgo = SignAlgo(
    name = "ecdsa_secp256k1_sha256",
    generateKeyPair = ecdsa_secp256k1_sha256.generateKeyPair,
    signer = kp ⇒
      Signer(
        kp.publicKey,
        new Crypto.Func[ByteVector, signature.Signature] {
          override def apply[F[_]](
            input: ByteVector
          )(implicit F: Monad[F]): EitherT[F, CryptoError, signature.Signature] =
            ecdsa_secp256k1_sha256.sign(kp, input)
        }
    ),
    checker = pk ⇒
      new SignatureChecker {
        override def check[F[_]: Monad](
          signature: fluence.crypto.signature.Signature,
          plain: ByteVector
        ): EitherT[F, CryptoError, Unit] =
          ecdsa_secp256k1_sha256.verify(pk, signature, plain)
    }
  )
}
