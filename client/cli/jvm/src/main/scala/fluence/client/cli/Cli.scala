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
import cats.Applicative
import com.typesafe.config.Config
import fluence.client.core.FluenceClient
import fluence.client.core.config.AesConfigParser
import fluence.crypto.{Crypto, KeyPair}
import fluence.crypto.aes.AesCrypt
import fluence.dataset.client.ClientDatasetStorageApi
import monix.eval.Task
import monix.execution.Scheduler.Implicits.global
import monix.reactive.Observable
import org.jline.reader.LineReaderBuilder
import org.jline.terminal.TerminalBuilder

import scala.language.higherKinds

object Cli extends slogging.LazyLogging {

  //for terminal improvements: history, navigation
  private val terminal = TerminalBuilder.terminal()
  private val lineReader = LineReaderBuilder.builder().terminal(terminal).build()
  private val readLine = IO(lineReader.readLine(Console.BLUE + "fluence" + Console.RESET + "< "))

  // TODO: what do it do? Why do we need it? Documentation needed
  def keyValueCryptoMethods[F[_]: Applicative](
    secretKey: KeyPair.Secret,
    config: Config
  ): Task[(Crypto.Cipher[String], Crypto.Cipher[String])] =
    for {
      aesConfig ← AesConfigParser.readAesConfigOrGetDefault[Task](config)
    } yield
      (
        AesCrypt.forString(secretKey.value, withIV = false, aesConfig),
        AesCrypt.forString(secretKey.value, withIV = true, aesConfig)
      )

  def restoreDataset(
    keyPair: KeyPair,
    fluenceClient: FluenceClient,
    config: Config,
    replicationN: Int
  ): Task[ClientDatasetStorageApi[Task, Observable, String, String]] =
    for {
      crypts ← keyValueCryptoMethods[Task](keyPair.secretKey, config)
      (keyCrypt, valueCrypt) = crypts
      dsOp ← fluenceClient.getDataset(keyPair, keyCrypt, valueCrypt)
      ds ← dsOp match {
        case Some(ds) ⇒ Task.pure(ds)
        case None ⇒
          logger.info("Contract not found, try to create new one.")
          fluenceClient.createNewContract(keyPair, replicationN, keyCrypt, valueCrypt)
      }
    } yield ds

  def handleCmds(ds: ClientDatasetStorageApi[Task, Observable, String, String], config: Config): IO[Boolean] =
    for {
      commandOp ← readLine.map(CommandParser.parseCommand)
      res ← commandOp match {
        case Some(CliOp.Exit) ⇒ // That's what it actually returns
          IO {
            logger.info("Exiting from fluence network.")
            false
          }

        case Some(CliOp.Put(k, v)) ⇒
          val t = for {
            _ ← ds.put(k, v)
            _ = logger.info("Success.")
          } yield true
          t.toIO

        case Some(CliOp.Get(k)) ⇒
          val t = for {
            res ← ds.get(k)
            printRes = res match {
              case Some(r) ⇒ "\"" + r + "\""
              case None ⇒ "<null>"
            }
            _ = logger.info("Result: " + printRes)
          } yield true
          t.toIO

        case Some(CliOp.Range(from, to)) ⇒
          val t = for {
            res ← ds.range(from, to).toListL
            _ = logger.info(s"Result: ${res.mkString("[", ",", "]")}")
          } yield true
          t.toIO

        case _ ⇒
          IO {
            logger.info("Wrong command.")
            true
          }
      }
    } yield res

}
