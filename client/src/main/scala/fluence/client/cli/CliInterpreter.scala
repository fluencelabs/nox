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
import cats.~>
import org.jline.reader.LineReaderBuilder
import org.jline.terminal.TerminalBuilder

class CliInterpreter() extends (CliOp ~> IO) {

  //for terminal improvements: history, navigation
  private val terminal = TerminalBuilder.terminal()
  private val lineReader = LineReaderBuilder.builder().terminal(terminal).build()

  import CliOp._

  override def apply[A](fa: CliOp[A]): IO[A] = ???
  //    fa match {
  //    case Exit =>
  //      println("Exit")
  //      IO.unit
  //
  //    case Get(key) =>
  //
  //    case Put(key, value) =>
  //
  //    case ReadLine(prefix) =>
  //
  //    case PrintLines(lines) =>
  //  }
}
