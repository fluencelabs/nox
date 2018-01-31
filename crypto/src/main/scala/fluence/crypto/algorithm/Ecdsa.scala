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
import cats.syntax.all._
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
//todo handle errors in all methods
class Ecdsa(curveType: String, scheme: String) extends SignatureFunctions {

  val ECDSA = "ECDSA"

  def nonFatalHandling[A, F[_]](a: ⇒ A)(errorText: String)(implicit F: MonadError[F, Throwable]): F[A] = {
    try F.pure(a)
    catch {
      case NonFatal(e) ⇒ F.raiseError(CryptoErr(errorText, Some(e)))
    }
  }

  override def generateKeyPair[F[_]](random: SecureRandom)(implicit F: MonadError[F, Throwable]): F[KeyPair] = {
    for {
      ecSpecOp ← F.pure(Option(ECNamedCurveTable.getParameterSpec(curveType)))
      ecSpec ← ecSpecOp match {
        case Some(ecs) ⇒ F.pure(ecs)
        case None      ⇒ F.raiseError[ECNamedCurveParameterSpec](CryptoErr("Parameter spec for the curve is not available"))
      }
      g ← nonFatalHandling(KeyPairGenerator.getInstance(ECDSA, Providers.BouncyCastle))("Cannot get KeyPairGenerator instance")
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
    for {
      spec ← F.pure(new PKCS8EncodedKeySpec(keyPair.secretKey.value.toArray))
      factory ← nonFatalHandling(KeyFactory.getInstance(ECDSA))("Cannot get key factory instance")
      ecdsaSign ← nonFatalHandling(Signature.getInstance(scheme, Providers.BouncyCastle))("Cannot get signature instance")
      _ ← nonFatalHandling(ecdsaSign.initSign(factory.generatePrivate(spec)))("Private key is invalid")
      _ ← nonFatalHandling(ecdsaSign.update(message.toArray))("Cannot update data to be signed")
    } yield fluence.crypto.signature.Signature(keyPair.publicKey, ByteVector(ecdsaSign.sign()))
  }

  override def verify[F[_]](signature: fluence.crypto.signature.Signature, message: ByteVector)(implicit F: MonadError[F, Throwable]): F[Boolean] = {
    for {
      spec ← F.pure(new X509EncodedKeySpec(signature.publicKey.value.toArray))
      factory ← nonFatalHandling(KeyFactory.getInstance(ECDSA))("Cannot get key factory instance")
      ecdsaVerify ← nonFatalHandling(Signature.getInstance(scheme, Providers.BouncyCastle))("Cannot get signature instance")
      _ ← nonFatalHandling(ecdsaVerify.initVerify(factory.generatePublic(spec)))("Public key is invalid")
      _ ← nonFatalHandling(ecdsaVerify.update(message.toArray))("Cannot update data to be verified")
    } yield ecdsaVerify.verify(signature.sign.toArray)
  }
}

object Ecdsa {
  val ecdsa_secp256k1_sha256 = new Ecdsa(Curves.secp256k1, "SHA256withECDSA")
}
