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

package fluence.grpc.proxy

import java.io.InputStream

import fluence.proxy.grpc.WebsocketMessage
import fluence.proxy.grpc.WebsocketMessage.Response
import io.grpc.{Status, StatusException}

object RequestConverter {

  def toEither(message: WebsocketMessage.Response): Either[StatusException, InputStream] = {
    message match {
      case Response.Payload(payload) ⇒ Right(payload.newInput())
      case Response.CompleteStatus(status) ⇒
        val ex = new StatusException(Status.fromCodeValue(status.code.value))
        Left(ex)
      case Response.Empty ⇒
        val ex = new StatusException(Status.UNKNOWN.withDescription("Empty response field."))
        Left(ex)
    }
  }
}
