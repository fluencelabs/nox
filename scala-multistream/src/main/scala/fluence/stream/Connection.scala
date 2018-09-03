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

/**
  * Interface that generalize connection between nodes.
  */
trait Connection {

  /**
    * BIDI streaming request.
    * @param service Protocol service name.
    * @param method Protocol method name.
    * @param requests Request stream.
    * @return Response stream.
    */
  def handle(
    service: String,
    method: String,
    requests: Observable[Array[Byte]]
  ): IO[Observable[Array[Byte]]]

  /**
    * One request - on response.
    * @param service Protocol service name.
    * @param method Protocol method name.
    * @param request Request for opposite node.
    * @return Response from opposite node.
    */
  def handleUnary(
    service: String,
    method: String,
    request: Array[Byte]
  ): IO[Array[Byte]]
}
