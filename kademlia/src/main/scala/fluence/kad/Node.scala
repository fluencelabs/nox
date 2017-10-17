package fluence.kad

import java.time.Instant

import cats.{ Order, Show }

case class Node[C](
    key:      Key,
    lastSeen: Instant,
    contact:  C
)

object Node {
  implicit def show[C](implicit ks: Show[Key], cs: Show[C]): Show[Node[C]] =
    n ⇒ s"Node(${ks.show(n.key)}, ${n.lastSeen}, ${cs.show(n.contact)})"

  def relativeOrder[C](key: Key): Order[Node[C]] = new Order[Node[C]] {
    private val order = Key.relativeOrder(key)

    override def compare(x: Node[C], y: Node[C]): Int = order.compare(x.key, y.key)
  }

  def relativeOrdering[C](key: Key): Ordering[Node[C]] =
    relativeOrder(key).compare(_, _)
}