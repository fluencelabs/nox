package fluence.kad

import scala.language.higherKinds

trait KademliaRPC[F[_], C] {

  def ping(): F[Node[C]]

  def lookup(key: Key): F[Seq[Node[C]]]

  def lookupIterative(key: Key): F[Seq[Node[C]]]

}
