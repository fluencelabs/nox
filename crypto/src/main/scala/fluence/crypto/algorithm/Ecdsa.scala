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

import java.security.spec.{ PKCS8EncodedKeySpec, X509EncodedKeySpec }
import java.security._

import cats.MonadError
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.crypto.keypair.KeyPair
import org.bouncycastle.jce.ECNamedCurveTable
import org.bouncycastle.jce.spec.ECNamedCurveParameterSpec
import scodec.bits.ByteVector

import scala.util.control.NonFatal

/**
 *
 * @param curveType http://www.bouncycastle.org/wiki/display/JA1/Supported+Curves+%28ECDSA+and+ECGOST%29
 * @param scheme https://bouncycastle.org/specifications.html
 */
class Ecdsa(curveType: String, scheme: String) extends SignatureFunctions {

  val ECDSA = "ECDSA"
  val BouncyCastleProvider = "BC"

  def nonFatalHandling[F[_], A](a: ⇒ A)(errorText: String)(implicit F: MonadError[F, Throwable]): F[A] = {
    try F.pure(a)
    catch {
      case NonFatal(e) ⇒ F.raiseError(CryptoErr(errorText))
    }
  }

  override def generateKeyPair[F[_]](random: SecureRandom)(implicit F: MonadError[F, Throwable]): F[KeyPair] = {
    for {
      ecSpecOp ← F.pure(Option(ECNamedCurveTable.getParameterSpec(curveType)))
      ecSpec ← ecSpecOp match {
        case Some(ecs) ⇒ F.pure(ecs)
        case None      ⇒ F.raiseError[ECNamedCurveParameterSpec](CryptoErr("Parameter spec for the curve is not available"))
      }
      g ← nonFatalHandling(getKeyPairGenerator)("Cannot get KeyPairGenerator instance")
      _ ← nonFatalHandling(g.initialize(ecSpec, random))("Could not initialize KeyPairGenerator")
      keyPair ← Option(g.generateKeyPair()) match {
        case Some(p) ⇒ F.pure(p)
        case None    ⇒ F.raiseError[java.security.KeyPair](CryptoErr("Could not generate KeyPair. Unexpected."))
      }
    } yield KeyPair(KeyPair.Public(ByteVector(keyPair.getPublic.getEncoded)), KeyPair.Secret(ByteVector(keyPair.getPrivate.getEncoded)))
  }

  override def generateKeyPair[F[_]]()(implicit F: MonadError[F, Throwable]): F[KeyPair] = {
    generateKeyPair(new SecureRandom())
  }

  override def sign[F[_]](keyPair: KeyPair, message: ByteVector)(implicit F: MonadError[F, Throwable]): F[fluence.crypto.signature.Signature] = {
    F.catchNonFatal(signMessage(keyPair.secretKey.value.toArray, message.toArray))
      .map(bb ⇒ fluence.crypto.signature.Signature(keyPair.publicKey, ByteVector(bb)))
  }

  override def verify[F[_]](signature: fluence.crypto.signature.Signature, message: ByteVector)(implicit F: MonadError[F, Throwable]): F[Boolean] = {
    F.catchNonFatal(verifySign(signature.publicKey.value.toArray, message.toArray, signature.sign.toArray))
  }

  private def keyInstance[F[_]](implicit F: MonadError[F, Throwable]) =
    nonFatalHandling(KeyFactory.getInstance(ECDSA))("Cannot get key factory instance")

  private def signMessage(privateKey: Array[Byte], message: Array[Byte]): Array[Byte] = {
    val keySpec = new PKCS8EncodedKeySpec(privateKey)

    val keyFactory = getKeyFactory
    val signProvider = getSignatureProvider

    try {
      signProvider.initSign(keyFactory.generatePrivate(keySpec))
      signProvider.update(message)
      signProvider.sign()
    } catch {
      case e: Throwable ⇒ throw CryptoErr(s"Cannot sign message. ${e.getLocalizedMessage}")
    }
  }

  private def verifySign(publicKey: Array[Byte], message: Array[Byte], signature: Array[Byte]): Boolean = {
    val keySpec = new X509EncodedKeySpec(publicKey)

    val keyFactory = getKeyFactory
    val signProvider = getSignatureProvider

    try {
      signProvider.initVerify(keyFactory.generatePublic(keySpec))
      signProvider.update(message)
      signProvider.verify(signature)
    } catch {
      case e: Throwable ⇒ throw CryptoErr(s"Cannot sign message. ${e.getLocalizedMessage}")
    }
  }

  private def getKeyPairGenerator = {
    try {
      KeyPairGenerator.getInstance(ECDSA, BouncyCastleProvider)
    } catch {
      case e: Throwable ⇒ throw CryptoErr(s"Cannot get key pair generator. ${e.getLocalizedMessage}")
    }
  }

  private def getKeyFactory = {
    try {
      KeyFactory.getInstance(ECDSA, BouncyCastleProvider)
    } catch {
      case e: Throwable ⇒ throw CryptoErr(s"Cannot get key factory instance. ${e.getLocalizedMessage}")
    }
  }

  private def getSignatureProvider = {
    try {
      Signature.getInstance(scheme, BouncyCastleProvider)
    } catch {
      case e: Throwable ⇒ throw CryptoErr(s"Cannot get signature instance. ${e.getLocalizedMessage}")
    }
  }
}

object Ecdsa {
  val ecdsa_secp256k1_sha256 = new Ecdsa("secp256k1", "SHA256withECDSA")
}
