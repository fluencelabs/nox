package fluence.client

import fastparse.all._

object CommandParser {
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
  val putCommand = P(Start ~ "put" ~ string ~ string).map{ case (k, v) ⇒ Put(k, v) }
  val getCommand = P(Start ~ "get" ~ string).map(k ⇒ Get(k))

  val parserCli = P(exit | putCommand | getCommand)

  //todo improve escape characters in put and get commands
  def parseCommand(str: String): Option[Operation] = {
    parserCli.parse(str) match {
      case Parsed.Success(op, _) ⇒ Some(op)
      case _                     ⇒ None
    }
  }
}
