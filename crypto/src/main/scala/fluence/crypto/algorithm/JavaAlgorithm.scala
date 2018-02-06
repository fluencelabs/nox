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

import java.security.Security

import fluence.crypto.keypair.KeyPair
import org.bouncycastle.jce.provider.BouncyCastleProvider
import scodec.bits.ByteVector

import scala.language.{ higherKinds, implicitConversions }

/**
 * trait that initializes a JVM-specific provider to work with cryptography
 */
private[crypto] trait JavaAlgorithm extends Algorithm {
  JavaAlgorithm.addProvider
}

object JavaAlgorithm {
  /**
   * getEncoded methods return ASN.1 encoding in PKCS#8 standard for secret key and X.509 standard for public
   * ASN.1 serialization standard defined by ISO. https://en.wikipedia.org/wiki/Abstract_Syntax_Notation_One
   * PKCS #8 is a standard syntax for storing private key information. https://en.wikipedia.org/wiki/PKCS_8
   * X.509 is a standard that defines the format of public key certificates. https://en.wikipedia.org/wiki/X.509
   */
  implicit def jKeyPairToKeyPair(jKeyPair: java.security.KeyPair): KeyPair =
    KeyPair(KeyPair.Public(ByteVector(jKeyPair.getPublic.getEncoded)), KeyPair.Secret(ByteVector(jKeyPair.getPrivate.getEncoded)))

  /**
   * add JVM-specific security provider in class loader
   */
  private lazy val addProvider = {
    Option(Security.getProvider(BouncyCastleProvider.PROVIDER_NAME))
      .foreach(_ ⇒ Security.removeProvider(BouncyCastleProvider.PROVIDER_NAME))
    Security.addProvider(new BouncyCastleProvider())
  }
}
