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
import fluence.client.FluenceClient
import fluence.crypto.keypair.KeyPair
import monix.execution.Scheduler.Implicits.global
import org.jline.reader.LineReaderBuilder
import org.jline.terminal.TerminalBuilder

import scala.language.higherKinds

object Cli extends slogging.LazyLogging {

  //for terminal improvements: history, navigation
  private val terminal = TerminalBuilder.terminal()
  private val lineReader = LineReaderBuilder.builder().terminal(terminal).build()
  private val readLine = IO(lineReader.readLine("fluence< "))

  def handleCmds(fluenceClient: FluenceClient, kp: KeyPair): IO[Boolean] =
    readLine.map(CommandParser.parseCommand).flatMap {
      case Some(CliOp.Exit) ⇒ // That's what it actually returns
        IO {
          logger.info("Exiting from fluence network.")
          false
        }

      case Some(CliOp.Put(k, v)) ⇒
        val t = for {
          ds ← fluenceClient.getOrCreateDataset(kp)
          _ ← ds.put(k, v)
          _ = logger.info("Success.")
        } yield true
        t.toIO

      case Some(CliOp.Get(k)) ⇒
        val t = for {
          ds ← fluenceClient.getOrCreateDataset(kp)
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

}
