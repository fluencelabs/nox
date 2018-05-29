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

import cats.effect.IO
import com.typesafe.config.{Config, ConfigFactory, ConfigRenderOptions}
import fluence.client.core.FluenceClient
import fluence.client.core.config.{AesConfigParser, KademliaConfigParser, KeyPairConfig, SeedsConfig}
import fluence.client.grpc.ClientGrpcServices
import fluence.crypto.aes.AesCrypt
import fluence.crypto.ecdsa.Ecdsa
import fluence.crypto.hash.JdkCryptoHasher
import fluence.crypto.keystore.FileKeyStorage
import fluence.crypto.signature.SignAlgo
import fluence.crypto.{Crypto, KeyPair}
import fluence.transport.grpc.client.GrpcClient
import monix.eval.Task
import monix.execution.Scheduler.Implicits.global
import org.jline.reader.LineReaderBuilder
import org.jline.terminal.TerminalBuilder
import slogging.MessageFormatter.PrefixFormatter
import slogging._

import scala.language.higherKinds

object ClientCliApp extends App with slogging.LazyLogging {

  // Pretty logger
  PrintLoggerFactory.formatter = new PrefixFormatter {
    override def formatPrefix(level: MessageLevel, name: String): String = s"[${Console.BLUE}fluence${Console.RESET}] "
  }
  LoggerConfig.factory = PrintLoggerFactory()
  LoggerConfig.level = LogLevel.DEBUG

  val algo: SignAlgo = Ecdsa.signAlgo
  import algo.checker
  val hasher: Crypto.Hasher[Array[Byte], Array[Byte]] = JdkCryptoHasher.Sha256
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
      kp ← FileKeyStorage.getKeyPair(kpConf.keyPath, algo)
    } yield kp

  /**
   * Initiation of cryptographic algorithms for key and value encryption depending on the config parameters.
   *
   * @param secretKey Secret key for dataset.
   * @param config Main configuration.
   * @return Tuple of cryptographic algorithms for key and value.
   */
  def keyValueCryptoMethods(
    secretKey: KeyPair.Secret,
    config: Config
  ): IO[(Crypto.Cipher[String], Crypto.Cipher[String])] =
    for {
      aesConfig ← AesConfigParser.readAesConfigOrGetDefault[IO](config)
    } yield
      (
        AesCrypt.forString(secretKey.value, withIV = false, aesConfig),
        AesCrypt.forString(secretKey.value, withIV = false, aesConfig)
      )

  /**
   * Method to get commands from user interface.
   */
  val readLineOp =
    for {
      terminal ← IO(TerminalBuilder.terminal())
      lineReader ← IO(LineReaderBuilder.builder().terminal(terminal).build())
    } yield lineReader.readLine(Console.BLUE + "fluence" + Console.RESET + "< ")

  /**
   * Command handler.
   */
  val cliHandler = new CommandHandler(readLineOp)

  val cliOp = for {
    fluenceClient ← buildClient
    keyPair ← getKeyPair(config, algo)
    keyValueCryptography ← keyValueCryptoMethods(keyPair.secretKey, config)
    (keyCrypto, valueCrypto) = keyValueCryptography
    _ = logger.debug("Going to restore client's datset...")
    dataset ← fluenceClient.restoreDataset(keyPair, keyCrypto, valueCrypto, replicationN = 2).toIO
    _ ← {
      lazy val handle: IO[Unit] =
        cliHandler.handleCmds(dataset).attempt.flatMap {
          case Right(true) ⇒ handle
          case Right(false) ⇒ IO.unit
          case Left(t) ⇒
            logger.error(s"Error while handling a command, cause=$t")
            handle
        }
      handle
    }
  } yield {}

  // Run Command Line Interface
  cliOp.unsafeRunSync()

}
