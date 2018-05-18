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

package fluence.crypto.keystore

import java.io.File
import java.nio.file.Files

import cats.syntax.applicativeError._
import cats.effect.IO
import fluence.codec.PureCodec
import fluence.crypto.{signature, KeyPair}

import scala.language.higherKinds
import scala.util.control.NonFatal

/**
 * File based storage for crypto keys.
 *
 * @param file Path to keys in file system
 */
class FileKeyStorage(file: File) extends slogging.LazyLogging {
  import KeyStore._

  private val codec = PureCodec[KeyPair, String]

  private val readFile: IO[String] =
    IO(Files.readAllBytes(file.toPath)).map(new String(_))

  val readKeyPair: IO[KeyPair] = readFile.flatMap(codec.inverse.runF[IO])

  private def writeFile(data: String): IO[Unit] = IO {
    logger.info("Storing secret key to file: " + file)
    if (!file.getParentFile.exists()) {
      logger.info(s"Parent directory does not exist: ${file.getParentFile}, trying to create")
      Files.createDirectories(file.getParentFile.toPath)
    }
    if (!file.exists()) file.createNewFile() else throw new RuntimeException(file.getAbsolutePath + " already exists")
    Files.write(file.toPath, data.getBytes)
  }

  def storeKeyPair(keyPair: KeyPair): IO[Unit] =
    codec.direct.runF[IO](keyPair).flatMap(writeFile)

  def readOrCreateKeyPair(createKey: IO[KeyPair]): IO[KeyPair] =
    readKeyPair.recoverWith {
      case NonFatal(e) ⇒
        logger.debug(s"KeyPair can't be loaded from $file, going to generate new keys", e)
        for {
          ks ← createKey
          _ ← storeKeyPair(ks)
        } yield ks
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
  def getKeyPair(keyPath: String, algo: signature.SignAlgo): IO[KeyPair] =
    IO(new FileKeyStorage(new File(keyPath)))
      .flatMap(_.readOrCreateKeyPair(algo.generateKeyPair.runF[IO](None)))

}
