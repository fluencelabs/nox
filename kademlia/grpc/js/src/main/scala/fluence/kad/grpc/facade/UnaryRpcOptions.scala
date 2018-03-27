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

package fluence.kad.grpc.facade

import scala.scalajs.js

/**
 * Options for unary grpc method
 * @tparam Req Type of request
 * @tparam Resp Type of response
 */
@js.native
trait UnaryRpcOptions[Req <: js.Any, Resp <: js.Any] extends js.Object {
  val request: Req = js.native
  val host: String = js.native
  val metadata = js.native
  val onEnd: UnaryOutput[Resp] ⇒ Unit = js.native
  val debug = js.native
}

object UnaryRpcOptions {

  /**
   *
   * @param onEnd Callback for handling response
   * @tparam Req Type of request
   * @tparam Resp Type of response
   */
  def apply[Req <: js.Any, Resp <: js.Any](
    request: Req,
    host: String,
    metadata: js.Any,
    onEnd: UnaryOutput[Resp] ⇒ Unit,
    debug: Boolean = false
  ): UnaryRpcOptions[Req, Resp] = {
    js.Dynamic
      .literal(request = request, host = host, metadata = metadata, onEnd = onEnd, debug = debug)
      .asInstanceOf[UnaryRpcOptions[Req, Resp]]
  }
}
