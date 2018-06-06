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

import org.scalajs.dom.html.TextArea
import slogging._
import slogging.LoggingUtils.argsBracketFormat

import scala.collection.mutable

/**
 * Logger, that will append logs to text area.
 *
 * @param lineLimit Limit of log lines in text area.
 */
class TextAreaLogger(textarea: TextArea, lineLimit: Int) extends AbstractUnderlyingLogger {

  val queue = new mutable.Queue[String]()

  @inline
  final def prefix(level: String, src: String) = s"[$level, $src]"

  @inline
  final def msg(level: String, src: String, msg: String) = s"${prefix(level, src)} $msg"

  @inline
  final def msg(level: String, src: String, msg: String, cause: Throwable) = s"${prefix(level, src)} $msg\n    $cause"

  @inline
  final def msg(level: String, src: String, msg: String, args: Any*) =
    s"${prefix(level, src)} ${argsBracketFormat(msg, args)}"

  private def addMessage(message: String): Unit = {
    if (queue.size >= lineLimit) {
      queue.dequeue()
    }
    queue.enqueue(message)

    textarea.value = queue.mkString("\n")
    textarea.scrollTop = textarea.scrollHeight
  }

  override def error(source: String, message: String): Unit = addMessage(msg("ERROR", source, message))

  override def error(source: String, message: String, cause: Throwable): Unit =
    addMessage(msg("ERROR", source, message, cause))

  override def error(source: String, message: String, args: Any*): Unit =
    addMessage(msg("ERROR", source, message, args))

  override def warn(source: String, message: String): Unit = addMessage(msg("WARN", source, message))

  override def warn(source: String, message: String, cause: Throwable): Unit =
    addMessage(msg("WARN", source, message, cause))

  override def warn(source: String, message: String, args: Any*): Unit = addMessage(msg("WARN", source, message, args))

  override def info(source: String, message: String): Unit = addMessage(msg("INFO", source, message))

  override def info(source: String, message: String, cause: Throwable): Unit =
    addMessage(msg("INFO", source, message, cause))

  override def info(source: String, message: String, args: Any*): Unit = addMessage(msg("INFO", source, message, args))

  override def debug(source: String, message: String): Unit = addMessage(msg("DEBUG", source, message))

  override def debug(source: String, message: String, cause: Throwable): Unit =
    addMessage(msg("DEBUG", source, message, cause))

  override def debug(source: String, message: String, args: Any*): Unit =
    addMessage(msg("DEBUG", source, message, args))

  override def trace(source: String, message: String): Unit = addMessage(msg("TRACE", source, message))

  override def trace(source: String, message: String, cause: Throwable): Unit =
    addMessage(msg("TRACE", source, message, cause))

  override def trace(source: String, message: String, args: Any*): Unit =
    addMessage(msg("TRACE", source, message, args))
}

class TextAreaWithConsoleLoggerFactory(textArea: TextArea, lineLimit: Int) extends UnderlyingLoggerFactory {

  val textLogger = new TextAreaLogger(textArea, lineLimit)

  override def getUnderlyingLogger(name: String): UnderlyingLogger = {

    MultiLogger(textLogger, PrintLoggerFactory.getUnderlyingLogger(name))
  }
}
