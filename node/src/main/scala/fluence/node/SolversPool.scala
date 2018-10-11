package fluence.node

import scala.language.higherKinds

class SolversPool[F[_]] {

}

object SolversPool{
  def apply[F[_]]: F[SolversPool[F]] = {
    ???
  }
}