package fluence.info

import scala.language.higherKinds

trait NodeInfoRpc[F[_]] {
  def getInfo(): F[NodeInfo]
}
