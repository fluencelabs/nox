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
import scala.scalajs.js.annotation.JSImport

@js.native
@JSImport("grpc-web-client", "grpc")
object Grpc extends js.Object {

  def unary[Req <: js.Any, Resp <: js.Any](
    methodDescriptor: MethodDescriptor[Req, Resp],
    props: UnaryRpcOptions[Req, Resp]
  ): js.Any =
    js.native

  def invoke[Req <: js.Any, Resp <: js.Any](
    methodDescriptor: MethodDescriptor[Req, Resp],
    props: InvokeRpcOptions[Req, Resp]
  ): js.Any =
    js.native
  def client: js.Any = js.native
}
