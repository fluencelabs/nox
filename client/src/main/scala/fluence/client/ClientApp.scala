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

package fluence.client

import cats.Apply
import cats.effect.IO
import com.typesafe.config.{ Config, ConfigFactory }
import fluence.client.cli.Cli
import fluence.crypto.SignAlgo
import fluence.crypto.algorithm.Ecdsa
import fluence.crypto.hash.{ CryptoHasher, JdkCryptoHasher }
import monix.execution.Scheduler.Implicits.global
import slogging.MessageFormatter.PrefixFormatter
import slogging._

import scala.language.higherKinds

object ClientApp extends App with slogging.LazyLogging {

  // Pretty logger
  PrintLoggerFactory.formatter = new PrefixFormatter {
    override def formatPrefix(level: MessageLevel, name: String): String = "[fluence] "
  }
  LoggerConfig.factory = PrintLoggerFactory()
  LoggerConfig.level = LogLevel.INFO

  val algo: SignAlgo = Ecdsa.signAlgo
  val hasher: CryptoHasher[Array[Byte], Array[Byte]] = JdkCryptoHasher.Sha256
  val config: Config = ConfigFactory.load()

  // Run Command Line Interface
  Apply[IO].map2(
    ClientComposer.buildClient(config, algo, hasher),
    ClientComposer.getKeyPair(config, algo)
  ){
      (fluenceClient, keyPair) ⇒
        Cli.restoreDataset(keyPair, fluenceClient, config, replicationN = 2).toIO
    }.flatMap(identity).flatMap {
      ds ⇒
        lazy val handle: IO[Unit] =
          Cli.handleCmds(ds, config)
            .attempt
            .flatMap{
              case Right(true)  ⇒ handle
              case Right(false) ⇒ IO.unit
              case Left(t) ⇒
                logger.error("Error while handling a command", t)
                handle
            }
        handle
    }
    .unsafeRunSync()

}
