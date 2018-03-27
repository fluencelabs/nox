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

package fluence.kad.grpc

import fluence.kad.grpc.facade.{Grpc, MethodDescriptor, UnaryOutput, UnaryRpcOptions}

import scala.concurrent.{Future, Promise}
import scala.scalajs.js
import scala.util.Try

class GrpcJSService(host: String, debug: Boolean = false) extends slogging.LazyLogging {

  val grpc: Grpc.type = Grpc

  def unary[Req <: js.Any, Resp <: js.Any](descriptor: MethodDescriptor[Req, Resp], req: Req): Future[Resp] = {
    val promise = Promise[Resp]

    val onEnd: UnaryOutput[Resp] ⇒ Unit = { out ⇒
      promise.tryComplete {
        Try {
          logger.debug(s"Response on $descriptor with status: ${out.status}")
          out.message
        }
      }
    }

    val options = UnaryRpcOptions[Req, Resp](
      req,
      host,
      "", //TODO add metadata facade and add metadata to api
      onEnd,
      debug = debug
    )
    grpc.unary[Req, Resp](descriptor, options)

    promise.future
  }
}
