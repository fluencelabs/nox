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
import fluence.dataset.client.ClientDatasetStorageApi
import monix.eval.Task
import monix.execution.Scheduler
import monix.reactive.Observable

class CommandHandler(readLineOperation: IO[String])(implicit scheduler: Scheduler) extends slogging.LazyLogging {

  def handleCmds(ds: ClientDatasetStorageApi[Task, Observable, String, String]): IO[Boolean] =
    for {
      commandOp ← readLineOperation.map(CommandParser.parseCommand)
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
