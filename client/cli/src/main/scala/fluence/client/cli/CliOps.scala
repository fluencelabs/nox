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

import cats.InjectK
import cats.free.Free.inject
import cats.free.Free

import scala.language.higherKinds

// TODO: it's actually not used, should either implement or remove
class CliOps[F[_]](implicit I: InjectK[CliOp, F]) {

  def exit: Free[F, Unit] = inject[CliOp, F](CliOp.Exit)

  def get(key: String): Free[F, Unit] = inject[CliOp, F](CliOp.Get(key))

  def put(key: String, value: String): Free[F, Unit] = inject[CliOp, F](CliOp.Put(key, value))

  def readLine(prefix: String): Free[F, String] = inject[CliOp, F](CliOp.ReadLine(prefix))

  def readPassword(prefix: String): Free[F, String] = inject[CliOp, F](CliOp.ReadLine(prefix))

  def print(lines: String*): Free[F, Unit] = inject[CliOp, F](CliOp.PrintLines(lines))
}
