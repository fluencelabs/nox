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

import fastparse.all._

object CommandParser {
  import CliOp._

  lazy private val parserCli = {
    val escape = P("\\" ~ CharIn("\"/\\bfnrt"))
    val space = P(CharsWhileIn(" \r\n").?)

    //get chars until quote
    def stringchar(quote: String): P0 = P(CharsWhile(!s"\\\n${quote(0)}".contains(_)))
    //get quote with chars if it is escaped
    def stringitem(quote: String): P0 = P(stringchar(quote) | escape)

    def string0(delimiter: String) = P(space ~ delimiter ~ stringitem(delimiter).rep.! ~ delimiter)
    //for string without quotes
    def stringSpace = P(space ~ stringitem(" ").rep(1).! ~ (space | End))

    val string: P[String] = P(string0("'") | string0("\"") | stringSpace)

    val exit = P(Start ~ "exit").map(_ ⇒ Exit)
    val putCommand = P(Start ~ "put" ~ string ~ string).map { case (k, v) ⇒ Put(k, v) }
    val getCommand = P(Start ~ "get" ~ string).map(k ⇒ Get(k))

    P(exit | putCommand | getCommand)
  }

  //todo improve escape characters in put and get commands
  def parseCommand(str: String): Option[CliOp[_]] = {
    parserCli.parse(str) match {
      case Parsed.Success(op, _) ⇒ Some(op)
      case _                     ⇒ None
    }
  }
}
