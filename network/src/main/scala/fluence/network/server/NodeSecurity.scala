package fluence.network.server

import cats.Applicative
import cats.syntax.applicative._
import cats.syntax.eq._
import fluence.kad.{ Key, Node }
import fluence.network.Contact

import scala.language.higherKinds

object NodeSecurity {

  /**
   * Checks if a node can be saved to RoutingTable
   * TODO: crypto checks
   *
   * @param acceptLocal If true, local addresses will be accepted; should be used only in testing
   * @tparam F Effect
   * @return Function to be called for each node prior to updating RoutingTable; returns F[true] if checks passed
   */
  def canBeSaved[F[_] : Applicative](self: Key, acceptLocal: Boolean): Node[Contact] ⇒ F[Boolean] =
    node ⇒ {
      if (node.key === self) false.pure[F]
      else (acceptLocal || !node.contact.isLocal).pure[F]
    }

}
