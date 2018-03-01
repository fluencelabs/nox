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

import cats.{ Applicative, MonadError }
import cats.effect.IO
import com.typesafe.config.Config
import fluence.client.FluenceClient
import fluence.client.config.AesConfigParser
import fluence.crypto.algorithm.AesCrypt
import fluence.crypto.keypair.KeyPair
import fluence.dataset.client.ClientDatasetStorageApi
import monix.eval.Task
import monix.execution.Scheduler.Implicits.global
import org.jline.reader.LineReaderBuilder
import org.jline.terminal.TerminalBuilder

import scala.language.higherKinds

object Cli extends slogging.LazyLogging {

  //for terminal improvements: history, navigation
  private val terminal = TerminalBuilder.terminal()
  private val lineReader = LineReaderBuilder.builder().terminal(terminal).build()
  private val readLine = IO(lineReader.readLine("fluence< "))

  def cryptoMethods[F[_] : Applicative](secretKey: KeyPair.Secret, config: Config)(implicit F: MonadError[F, Throwable]): Task[(AesCrypt[F, String], AesCrypt[F, String])] = {
    for {
      aesConfig ← AesConfigParser.readAesConfigOrGetDefault[Task](config)
    } yield (AesCrypt.forString(secretKey.value, withIV = false, aesConfig), AesCrypt.forString(secretKey.value, withIV = true, aesConfig))
  }

  def restoreDataset(keyPair: KeyPair, fluenceClient: FluenceClient, config: Config, replicationN: Int): Task[ClientDatasetStorageApi[Task, String, String]] = {
    for {
      crypts ← cryptoMethods[Task](keyPair.secretKey, config)
      (keyCrypt, valueCrypt) = crypts
      dsOp ← fluenceClient.getDataset(keyPair, keyCrypt, valueCrypt)
      ds ← dsOp match {
        case Some(ds) ⇒ Task.pure(ds)
        case None ⇒
          logger.info("Contract not found, try to create new one.")
          fluenceClient.createNewContract(keyPair, replicationN, keyCrypt, valueCrypt)
      }
    } yield ds
  }

  def handleCmds(ds: ClientDatasetStorageApi[Task, String, String], config: Config): IO[Boolean] =
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
              case None    ⇒ "<null>"
            }
            _ = logger.info("Result: " + printRes)
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
