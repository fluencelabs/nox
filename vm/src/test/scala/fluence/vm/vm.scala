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

package fluence

import cats.data.EitherT
import cats.effect.IO

import scala.concurrent.duration.Duration
import scala.concurrent.duration._

package object vm {

  implicit class EitherTValueReader[E, V](origin: EitherT[IO, E, V]) {

    def success(timeout: Duration = 3.seconds): V =
      origin.value.unsafeRunTimed(timeout).get.right.get

    def failed(timeout: Duration = 3.seconds): E =
      origin.value.unsafeRunTimed(timeout).get.left.get
  }

}
