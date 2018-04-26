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

package fluence.client.cli

import cats.Apply
import cats.effect.IO
import com.typesafe.config.{Config, ConfigFactory, ConfigRenderOptions}
import fluence.client.core.FluenceClient
import fluence.client.core.config.{KademliaConfigParser, KeyPairConfig, SeedsConfig}
import fluence.client.grpc.ClientGrpcServices
import fluence.crypto.algorithm.Ecdsa
import fluence.crypto.hash.{CryptoHasher, JdkCryptoHasher}
import fluence.crypto.signature.SignAlgo
import fluence.crypto.{FileKeyStorage, KeyPair}
import fluence.transport.grpc.client.GrpcClient
import monix.eval.Task
import monix.execution.Scheduler.Implicits.global
import slogging.MessageFormatter.PrefixFormatter
import slogging._

import scala.language.higherKinds

object ClientCliApp extends App with slogging.LazyLogging {

  // Pretty logger
  PrintLoggerFactory.formatter = new PrefixFormatter {
    override def formatPrefix(level: MessageLevel, name: String): String = "[fluence] "
  }
  LoggerConfig.factory = PrintLoggerFactory()
  LoggerConfig.level = LogLevel.INFO

  val algo: SignAlgo = Ecdsa.signAlgo
  import algo.checker
  val hasher: CryptoHasher[Array[Byte], Array[Byte]] = JdkCryptoHasher.Sha256
  val config: Config = ConfigFactory.load()

  logger.debug(
    "Client config is :" +
      config.getConfig("fluence").root().render(ConfigRenderOptions.defaults().setOriginComments(false))
  )

  val client = ClientGrpcServices.build[Task](GrpcClient.builder)

  val buildClient: IO[FluenceClient] = for {
    seedsConf ← SeedsConfig.read(config)
    contacts ← seedsConf.contacts
    kadConfig ← KademliaConfigParser.readKademliaConfig[IO](config)
    client ← FluenceClient.build(contacts, algo, hasher, kadConfig, client)
  } yield client

  def getKeyPair(config: Config, algo: SignAlgo): IO[KeyPair] =
    for {
      kpConf ← KeyPairConfig.read(config)
      kp ← FileKeyStorage.getKeyPair[IO](kpConf.keyPath, algo)
    } yield kp

  // Run Command Line Interface
  Apply[IO]
    .map2(
      buildClient,
      getKeyPair(config, algo)
    ) { (fluenceClient, keyPair) ⇒
      Cli.restoreDataset(keyPair, fluenceClient, config, replicationN = 2).toIO
    }
    .flatMap(identity)
    .flatMap { ds ⇒
      lazy val handle: IO[Unit] =
        Cli.handleCmds(ds, config).attempt.flatMap {
          case Right(true) ⇒ handle
          case Right(false) ⇒ IO.unit
          case Left(t) ⇒
            logger.error(s"Error while handling a command, cause=$t")
            handle
        }
      handle
    }
    .unsafeRunSync()

}
