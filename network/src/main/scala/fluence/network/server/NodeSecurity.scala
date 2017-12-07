package fluence.network.server

import cats.Applicative
import cats.syntax.applicative._
import fluence.kad.Node
import fluence.network.Contact

import scala.language.higherKinds

object NodeSecurity {

  /**
   * Checks if Node[Contact] is correct in terms of IP accessibility and signatures so that it's safe to save it
   * into RoutingTable
   *
   * @param acceptLocal Set to true to test network on localhost
   * @tparam F Effect
   * @return Function to be called on each node prior to saving it to RoutingTable; returns F[ true ] if it's safe to save it
   */
  def canBeSaved[F[_] : Applicative](acceptLocal: Boolean): Node[Contact] ⇒ F[Boolean] =
    node ⇒ {
      (acceptLocal || !node.contact.isLocal).pure[F]
    }

}
