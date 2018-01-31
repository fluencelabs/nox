package fluence.info

import scala.language.higherKinds

class NodeInfoService[F[_]](nodeInfo: () ⇒ F[NodeInfo]) extends NodeInfoRpc[F] {
  override def getInfo(): F[NodeInfo] = nodeInfo()
}
