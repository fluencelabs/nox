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
