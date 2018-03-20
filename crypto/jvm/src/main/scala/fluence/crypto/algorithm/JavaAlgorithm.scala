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

import org.bouncycastle.jce.provider.BouncyCastleProvider

import scala.language.{higherKinds, implicitConversions}

/**
 * trait that initializes a JVM-specific provider to work with cryptography
 */
private[crypto] trait JavaAlgorithm extends Algorithm {
  JavaAlgorithm.addProvider
}

object JavaAlgorithm {

  /**
   * add JVM-specific security provider in class loader
   */
  private lazy val addProvider = {
    Option(Security.getProvider(BouncyCastleProvider.PROVIDER_NAME))
      .foreach(_ ⇒ Security.removeProvider(BouncyCastleProvider.PROVIDER_NAME))
    Security.addProvider(new BouncyCastleProvider())
  }
}
