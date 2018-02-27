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
//0x03d46cdeb3b5fcde04b7dd799a93808fc59a1c5fb8fc342678138391f49e95ea22
//0x03d46cdeb3b5fcde04b7dd799a93808fc59a1c5fb8fc342678138391f49e95ea22
//eyJwayI6IkE5UnMzck8xX040RXQ5MTVtcE9BajhXYUhGLTRfRFFtZUJPRGtmU2VsZW9pIiwicHYiOjB9.eyJpcCI6IjE5Mi4xNjguMC4xNCIsImdwIjoxMTA0MiwiZ2giOiJkNTdiY2U1YzAwNWZjZDQzZTNkZDViNzZkNjM2NjU2OGMxZTBjODdhIn0=.MEUCIQDbDGme05WiBHdLyH6jzppKZ7TrzCf675UQK2GbQ3oosAIgOTswjdvP-w7jXoShqLzlI3Ku7GZcEeBRDgc5XhlQtBI=
//eyJwayI6IkE5UnMzck8xX040RXQ5MTVtcE9BajhXYUhGLTRfRFFtZUJPRGtmU2VsZW9pIiwicHYiOjB9.eyJpcCI6IjE5Mi4xNjguMC4xNCIsImdwIjoxMTA0MiwiZ2giOiJkNTdiY2U1YzAwNWZjZDQzZTNkZDViNzZkNjM2NjU2OGMxZTBjODdhIn0=.MEUCIQCKJHWfi3UlO0eALt0S9jgJVJiaegxUNseC0T9VmzZrlwIgQYMgPLnmw7n0crUJAE7YR2y5d0CjKmWuQVOlFlX51lc=
