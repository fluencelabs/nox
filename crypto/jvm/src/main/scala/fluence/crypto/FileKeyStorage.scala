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

import cats.MonadError
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.crypto.keypair.KeyPair
import io.circe.parser.decode
import io.circe.syntax._

import scala.language.higherKinds

class FileKeyStorage[F[_]](file: File)(implicit F: MonadError[F, Throwable]) {
  import KeyStore._
  def readKeyPair: F[KeyPair] = {
    val keyBytes = Files.readAllBytes(file.toPath)
    for {
      storageOp ← F.fromEither(decode[Option[KeyStore]](new String(keyBytes)))
      storage ← storageOp match {
        case None     ⇒ F.raiseError[KeyStore](new RuntimeException("Cannot parse file with keys."))
        case Some(ks) ⇒ F.pure(ks)
      }
    } yield storage.keyPair
  }

  def storeSecretKey(key: KeyPair): F[Unit] =
    F.catchNonFatal {
      if (!file.exists()) file.createNewFile() else throw new RuntimeException(file.getAbsolutePath + " already exists")
      val str = KeyStore(key).asJson.toString()

      Files.write(file.toPath, str.getBytes)
    }

  def getOrCreateKeyPair(f: ⇒ F[KeyPair]): F[KeyPair] =
    if (file.exists()) {
      readKeyPair
    } else {
      for {
        newKeys ← f
        _ ← storeSecretKey(newKeys)
      } yield newKeys
    }
}

object FileKeyStorage {
  /**
   * Generates or loads keypair
   *
   * @param keyPath Path to store keys in
   * @param algo Sign algo
   * @return Keypair, either loaded or freshly generated
   */
  def getKeyPair[F[_]](keyPath: String, algo: SignAlgo)(implicit F: MonadError[F, Throwable]): F[KeyPair] = {
    val keyFile = new File(keyPath)
    val keyStorage = new FileKeyStorage[F](keyFile)
    keyStorage.getOrCreateKeyPair(algo.generateKeyPair[F]().value.flatMap(F.fromEither))
  }
}
