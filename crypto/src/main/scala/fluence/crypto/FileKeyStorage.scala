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

package fluence.crypto

import java.io.File
import java.nio.file.Files
import java.security
import java.security.KeyFactory
import java.security.spec.PKCS8EncodedKeySpec

import cats.MonadError
import fluence.crypto.algorithm.{ Ecdsa, JavaAlgorithm }
import fluence.crypto.keypair.KeyPair
import org.bouncycastle.jce.spec.{ ECPrivateKeySpec, ECPublicKeySpec }

class FileKeyStorage[F[_]](file: File)(implicit F: MonadError[F, Throwable]) extends JavaAlgorithm {
  def readKeyPair: F[KeyPair] = {
    F.catchNonFatal {
      val keyBytes = Files.readAllBytes(file.toPath)
      val keySpec = new PKCS8EncodedKeySpec(keyBytes)

      val keyFactory = KeyFactory.getInstance(Ecdsa.ECDSA)
      val privateKey = keyFactory.generatePrivate(keySpec)

      //todo avoid classOf
      val privSpec = keyFactory.getKeySpec(privateKey, classOf[ECPrivateKeySpec])
      val params = privSpec.getParams

      val q = params.getG.multiply(privSpec.getD)

      val pubSpec = new ECPublicKeySpec(q, params)
      val pubKey = keyFactory.generatePublic(pubSpec)

      JavaAlgorithm.jKeyPairToKeyPair(new security.KeyPair(pubKey, privateKey))
    }
  }

  def storeSecretKey(key: KeyPair.Secret): F[Unit] = {
    import java.io.FileOutputStream
    F.catchNonFatal {
      if (!file.exists()) file.createNewFile() else throw new RuntimeException(file.getAbsolutePath + " already exists")
      val fos = new FileOutputStream(file)

      fos.write(key.value.toArray)
      fos.close()
    }
  }
}
