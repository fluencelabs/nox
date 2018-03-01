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

package fluence.node

import cats.effect.IO
import monix.eval.Task
import monix.execution.Scheduler.Implicits.global
import slogging.{ LogLevel, LoggerConfig, PrintLoggerFactory }

import scala.language.higherKinds

object NodeApp extends App with slogging.LazyLogging {

  // Simply log everything to stdout
  LoggerConfig.factory = PrintLoggerFactory()
  LoggerConfig.level = LogLevel.INFO

  logger.info("Going to run Fluence Server...")

  FluenceNode.startNode()
    .attempt
    .flatMap {
      case Left(t)  ⇒ IO.raiseError(t)
      case Right(_) ⇒ Task.never.toIO
    }
    .unsafeRunSync()
}
