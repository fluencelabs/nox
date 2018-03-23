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
import scala.scalajs.js.annotation.{JSGlobal, JSImport}
import scala.scalajs.js.typedarray.Uint8Array
import scala.scalajs.js.JSConverters._

@js.native
@JSImport("./generated/grpc_pb", "Node")
class Node(val array: js.Array[Uint8Array]) extends js.Object {

  def this(id: Uint8Array, contact: Uint8Array) = this(js.Array(id, contact))

  val id: Uint8Array = array.head
  val contact: Uint8Array = array.tail.head
}

@js.native
@JSImport("./generated/grpc_pb", "NodesResponse")
class NodesResponse(val nodes: Seq[Node]) extends js.Object

@js.native
@JSImport("./generated/grpc_pb", "PingRequest")
class PingRequest() extends js.Object

@js.native
@JSImport("./generated/grpc_pb", "LookupRequest")
class LookupRequest(val key: Uint8Array, val numberOfNodes: Int) extends js.Object {
  key.toJSArray.toArray.map(_.toByte)
}

@js.native
@JSImport("./generated/grpc_pb", "LookupAwayRequest")
class LookupAwayRequest(val key: Uint8Array, val moveAwayFrom: Uint8Array, val numberOfNodes: Int) extends js.Object {
  //key.toJSArray.toArray.map(_.toByte)
}

object Proto {
  val module = "./generated/grpc_pb"
}
