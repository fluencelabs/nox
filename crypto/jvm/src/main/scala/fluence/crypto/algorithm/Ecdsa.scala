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

package fluence.crypto.algorithm

import java.math.BigInteger
import java.security._
import java.security.interfaces.ECPrivateKey

import cats.MonadError
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.crypto.hash.{ CryptoHasher, JdkCryptoHasher }
import fluence.crypto.keypair.KeyPair
import org.bouncycastle.jce.ECNamedCurveTable
import org.bouncycastle.jce.interfaces.ECPublicKey
import org.bouncycastle.jce.provider.BouncyCastleProvider
import org.bouncycastle.jce.spec.{ ECNamedCurveParameterSpec, ECParameterSpec, ECPrivateKeySpec, ECPublicKeySpec }
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * Elliptic Curve Digital Signature Algorithm
 * @param curveType http://www.bouncycastle.org/wiki/display/JA1/Supported+Curves+%28ECDSA+and+ECGOST%29
 * @param scheme https://bouncycastle.org/specifications.html
 */
class Ecdsa(curveType: String, scheme: String, cryptoHasher: Option[CryptoHasher[Array[Byte], Array[Byte]]] = None) extends JavaAlgorithm
  with SignatureFunctions with KeyGenerator {
  import Ecdsa._
  import CryptoErr._

  val HEXradix = 16

  override def generateKeyPair[F[_]](seed: Option[Array[Byte]])(implicit F: MonadError[F, Throwable]): F[KeyPair] = {
    for {
      ecSpecOp ← F.pure(Option(ECNamedCurveTable.getParameterSpec(curveType)))
      ecSpec ← ecSpecOp match {
        case Some(ecs) ⇒ F.pure(ecs)
        case None      ⇒ F.raiseError[ECNamedCurveParameterSpec](CryptoErr("Parameter spec for the curve is not available."))
      }
      g ← getKeyPairGenerator
      _ ← nonFatalHandling(g.initialize(ecSpec, seed.map(new SecureRandom(_)).getOrElse(new SecureRandom())))("Could not initialize KeyPairGenerator.")
      keyPair ← Option(g.generateKeyPair()) match {
        case Some(p) ⇒
          //store S number for private key and compressed Q point on curve for public key
          val pk = ByteVector(p.getPublic.asInstanceOf[ECPublicKey].getQ.getEncoded(true))
          val bg = p.getPrivate.asInstanceOf[ECPrivateKey].getS
          val sk = ByteVector.fromValidHex(bg.toString(HEXradix))
          F.pure(KeyPair.fromByteVectors(pk, sk))
        case None ⇒ F.raiseError[KeyPair](CryptoErr("Could not generate KeyPair. Unexpected."))
      }
    } yield keyPair
  }

  override def sign[F[_]](keyPair: KeyPair, message: ByteVector)(implicit F: MonadError[F, Throwable]): F[fluence.crypto.signature.Signature] = {
    signMessage(new BigInteger(keyPair.secretKey.value.toHex, HEXradix), message.toArray)
      .map(bb ⇒ fluence.crypto.signature.Signature(keyPair.publicKey, ByteVector(bb)))
  }

  override def verify[F[_]](signature: fluence.crypto.signature.Signature, message: ByteVector)(implicit F: MonadError[F, Throwable]): F[Boolean] = {
    verifySign(signature.publicKey.value.toArray, message.toArray, signature.sign.toArray)
  }

  private def signMessage[F[_]](privateKey: BigInteger, message: Array[Byte])(implicit F: MonadError[F, Throwable]): F[Array[Byte]] = {
    for {
      ec ← curveSpec
      keySpec ← nonFatalHandling(new ECPrivateKeySpec(privateKey, ec))("Cannot read private key.")
      keyFactory ← getKeyFactory
      signProvider ← getSignatureProvider
      sign ← {
        nonFatalHandling {
          signProvider.initSign(keyFactory.generatePrivate(keySpec))
          signProvider.update(cryptoHasher.map(h ⇒ h.hash(message)).getOrElse(message))
          signProvider.sign()
        }("Cannot sign message.")
      }
    } yield sign
  }

  private def verifySign[F[_]](publicKey: Array[Byte], message: Array[Byte], signature: Array[Byte])(implicit F: MonadError[F, Throwable]): F[Boolean] = {
    for {
      ec ← curveSpec
      keySpec ← nonFatalHandling(new ECPublicKeySpec(ec.getCurve.decodePoint(publicKey), ec))("Cannot read public key.")
      keyFactory ← getKeyFactory
      signProvider ← getSignatureProvider
      verify ← {
        nonFatalHandling {
          signProvider.initVerify(keyFactory.generatePublic(keySpec))
          signProvider.update(cryptoHasher.map(h ⇒ h.hash(message)).getOrElse(message))
          signProvider.verify(signature)
        }("Cannot verify message.")
      }
    } yield verify
  }

  private def curveSpec[F[_]](implicit F: MonadError[F, Throwable]) =
    nonFatalHandling(ECNamedCurveTable.getParameterSpec(curveType).asInstanceOf[ECParameterSpec])("Cannot get curve parameters.")

  private def getKeyPairGenerator[F[_]](implicit F: MonadError[F, Throwable]) =
    nonFatalHandling(KeyPairGenerator.getInstance(ECDSA, BouncyCastleProvider.PROVIDER_NAME))("Cannot get key pair generator.")

  private def getKeyFactory[F[_]](implicit F: MonadError[F, Throwable]) =
    nonFatalHandling(KeyFactory.getInstance(ECDSA, BouncyCastleProvider.PROVIDER_NAME))("Cannot get key factory instance.")

  private def getSignatureProvider[F[_]](implicit F: MonadError[F, Throwable]) =
    nonFatalHandling(Signature.getInstance(scheme, BouncyCastleProvider.PROVIDER_NAME))("Cannot get signature instance.")
}

object Ecdsa {
  //algorithm name in security provider
  val ECDSA = "ECDSA"

  /**
   * size of key is 256 bit
   * `secp256k1` refers to the parameters of the ECDSA curve
   * `NONEwithECDSA with sha-256 hasher` Preferably the size of the key is greater than or equal to the digest algorithm
   */
  def ecdsa_secp256k1_sha256[F[_]](implicit F: MonadError[F, Throwable]) = new Ecdsa("secp256k1", "NONEwithECDSA", Some(JdkCryptoHasher.Sha256))
}
