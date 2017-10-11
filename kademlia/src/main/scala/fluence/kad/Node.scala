package fluence.kad

import cats.Order

case class Node[C](key: Key, contact: C)

object Node {
  def relativeOrder[C](key: Key): Order[Node[C]] = new Order[Node[C]] {
    private val order = Key.relativeOrder(key)

    override def compare(x: Node[C], y: Node[C]): Int = order.compare(x.key, y.key)
  }

  def relativeOrdering[C](key: Key): Ordering[Node[C]] =
    relativeOrder(key).compare(_, _)
}