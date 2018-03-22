package fluence.kad.grpc.facade

import scala.scalajs.js
import scala.scalajs.js.annotation.JSGlobal
import scala.scalajs.js.typedarray.Uint8Array
import scala.scalajs.js.JSConverters._

@js.native
@JSGlobal
class Node(id: Uint8Array, contact: Uint8Array)

@js.native
@JSGlobal
class NodeResponse(nodes: List[Node])

@js.native
@JSGlobal
class PingRequest()

@js.native
@JSGlobal
class LookupRequest(key: Uint8Array, numberOfNodes: Int) {
  key.toJSArray.toArray.map(_.toByte)
}

@js.native
@JSGlobal
class LookupAwayRequest(key: Uint8Array, moveAwayFrom: Uint8Array, numberOfNodes: Int) {
  //key.toJSArray.toArray.map(_.toByte)
}
