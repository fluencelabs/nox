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

import cats.{ Applicative, MonadError }
import cats.effect.IO
import com.typesafe.config.{ Config, ConfigFactory }
import fluence.crypto.KeyStore
import fluence.crypto.algorithm.{ AesCrypt, Ecdsa }
import fluence.crypto.hash.JdkCryptoHasher
import fluence.crypto.keypair.KeyPair
import fluence.dataset.client.ClientDatasetStorageApi
import fluence.kad.protocol.Contact
import io.circe.syntax._
import monix.eval.Task
import monix.execution.Scheduler.Implicits.global
import org.jline.reader.LineReaderBuilder
import org.jline.terminal.TerminalBuilder
import scodec.bits.{ Bases, ByteVector }
import slogging.MessageFormatter.PrefixFormatter
import slogging._

import scala.collection.JavaConverters._
import scala.language.higherKinds
import scala.util.Try

object ClientApp extends App with slogging.LazyLogging {
  import KeyStore._

  val alphabet = Bases.Alphabets.Base64Url

  PrintLoggerFactory.formatter = new PrefixFormatter {
    override def formatPrefix(level: MessageLevel, name: String): String = "[fluence] "
  }
  LoggerConfig.factory = PrintLoggerFactory()
  LoggerConfig.level = LogLevel.INFO

  val algo = Ecdsa.signAlgo
  import algo.checker

  //for terminal improvements: history, navigation
  val terminal = TerminalBuilder.terminal()
  val lineReader = LineReaderBuilder.builder().terminal(terminal).build()

  ArgsParser.parse(args) match {
    case Some(c) ⇒

      val config = c.config match {
        case Some(configFile) ⇒ ConfigFactory.parseFile(configFile)
        case None             ⇒ ConfigFactory.load()
      }

      val seedsB64 = if (c.seed.isEmpty) config.getStringList("fluence.network.join.seeds").asScala else c.seed

      val task = for {
        seeds ← Task.traverse(seedsB64.map(s ⇒ Contact.readB64seed[Task](s).value))(_.flatMap(e ⇒ Task.fromTry(e.toTry)))
        keyPair ← c.keyStore.map(ks ⇒ Task.pure(ks.keyPair))
          .orElse(Try(config.getString("fluence.client.keystore")).map(ks ⇒ KeyStore.fromBase64(ks).keyPair).toOption.map(Task.pure))
          .getOrElse(algo.generateKeyPair[Task]().value.flatMap(e ⇒ Task.fromTry(e.toTry)))
        _ = {
          logger.info("Your keypair is:")
          logger.info(Console.MAGENTA + ByteVector(KeyStore(keyPair).asJson.noSpaces.getBytes()).toBase64(alphabet) + Console.RESET)
          logger.info("Store it and use it for auth.")
          logger.info("Creating fluence client.")
        }
        fluenceClient ← FluenceClient(seeds, algo, JdkCryptoHasher.Sha256, config)
        _ = logger.info("Restore dataset from kademlia.")
        cryptoMethods ← cryptoMethods[Task](keyPair.secretKey, config)
        (keyCrypt, valueCrypt) = cryptoMethods
        ac = AuthorizedClient(keyPair)
        dsOp ← fluenceClient.getDataset(ac, keyCrypt, valueCrypt)
        ds ← dsOp match {
          case Some(ds) ⇒ Task.pure(ds)
          case None ⇒
            logger.info("Contract not found, try to create new one.")
            fluenceClient.createNewContract(ac, 2, keyCrypt, valueCrypt)
        }
      } yield (fluenceClient, ac, ds)

      val io = for {
        res ← task.toIO
        (flClient, ac, ds) = res
        _ = {
          logger.info("You can put or get data from remote node.")
          logger.info("Examples: ")
          logger.info("put \"some key\" \"value to put\"")
          logger.info("get \"some key\"")
        }
        _ ← handleCmds(flClient, ac, config, ds)
      } yield {}

      io.attempt
        .unsafeRunSync() match {
          case Left(err) ⇒
            err.printStackTrace()
            logger.error("Error", err)
          case Right(_) ⇒
            logger.info("Bye!")
        }

    case None ⇒
    //Scopt will generate error message when args is wrong:
    //Error: Unknown option --wrongoption
    //Try --help for more information.
  }

  def cryptoMethods[F[_] : Applicative](secretKey: KeyPair.Secret, config: Config)(implicit F: MonadError[F, Throwable]): Task[(AesCrypt[F, String], AesCrypt[F, String])] = {
    for {
      aesConfig ← AesConfigParser.readAesConfigOrGetDefault[Task](config)
    } yield (AesCrypt.forString(secretKey.value, withIV = false, aesConfig), AesCrypt.forString(secretKey.value, withIV = true, aesConfig))
  }

  def readLine = IO(lineReader.readLine("fluence< "))

  def handleCmds(fluenceClient: FluenceClient, ac: AuthorizedClient, config: Config, ds: ClientDatasetStorageApi[Task, String, String]): IO[Unit] = {
    lazy val handle: IO[Unit] = readLine.map(CommandParser.parseCommand).flatMap {
      case Some(Exit) ⇒
        IO(logger.info("Exiting from fluence network."))
      case Some(Put(k, v)) ⇒
        val t = for {
          _ ← ds.put(k, v)
          _ = logger.info("Success.")
        } yield ()
        t.toIO.flatMap(_ ⇒ handle)
      case Some(Get(k)) ⇒
        val t = for {
          res ← ds.get(k)
          printRes = res match {
            case Some(r) ⇒ "\"" + r + "\""
            case None    ⇒ "<null>"
          }
          _ = logger.info("Result: " + printRes)
        } yield ()
        t.toIO.flatMap(_ ⇒ handle)
      case _ ⇒
        IO(logger.info("Wrong command.")).flatMap(_ ⇒ handle)
    }

    handle
  }

}
