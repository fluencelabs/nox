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
import scala.util.{Failure, Try}

/**
 * Service for communication on the GRPC (http2) protocol from the browser
 * @param address Server address "host:port"
 * @param debug Debug info from js service
 */
class GrpcJSService(address: String, debug: Boolean = false) extends slogging.LazyLogging {

  private val grpc = Grpc

  /**
   * Single request - single response method
   * @param descriptor  Descriptor from generated code, describing the relationship between request and response
   * @param req Single request
   * @tparam Req Type of request
   * @tparam Resp Type of response
   */
  def unary[Req <: js.Any, Resp <: js.Any](descriptor: MethodDescriptor[Req, Resp], req: Req): Future[Resp] = {
    val promise = Promise[Resp]

    val onEnd: UnaryOutput[Resp] ⇒ Unit = { out ⇒
      promise.tryComplete {

        logger.debug(s"Response on $descriptor with status: ${out.status}")

        //TODO handle all response statuses https://godoc.org/google.golang.org/grpc/codes
        if (out.status == 0) Try(out.message)
        else if (out.status == 13) Failure(new RuntimeException("No connection"))
        else Failure(new RuntimeException(s"Status of response: ${out.status}"))
      }
    }

    val options = UnaryRpcOptions[Req, Resp](
      req,
      address,
      "", //TODO add metadata facade and add metadata to api
      onEnd,
      debug = debug
    )
    grpc.unary[Req, Resp](descriptor, options)

    promise.future
  }
}
