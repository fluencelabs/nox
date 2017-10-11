package fluence.kad

import scala.language.higherKinds

abstract class Kademlia[F[_], C] extends KademliaRPC[F, C]{
  def rpc: C => KademliaRPC[F, C]

  def update(node: Node[C]): F[Unit]

  def register(peers: Seq[C]): F[Unit]
}