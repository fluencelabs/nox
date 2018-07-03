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

package fluence.stream

import cats.effect.IO
import monix.reactive.Observable

trait StreamHandler {

  def handle(
    service: String,
    method: String,
    requests: Observable[Array[Byte]]
  ): IO[Observable[Array[Byte]]]

  def handleUnary(
    service: String,
    method: String,
    request: Array[Byte]
  ): IO[Array[Byte]]
}
