package fluence.kad

import java.time.Instant

import cats.{ Order, Show }

/**
 * Kademlia's Node representation
 * @param key Key
 * @param lastSeen When was the node last seen
 * @param contact Description on how to contact the node over network
 * @tparam C Contact info
 */
case class Node[C](
    key: Key,
    lastSeen: Instant,
    contact: C
)

object Node {
  implicit def show[C](implicit ks: Show[Key], cs: Show[C]): Show[Node[C]] =
    n ⇒ s"Node(${ks.show(n.key)}, ${n.lastSeen}, ${cs.show(n.contact)})"

  /**
   * Builds order by distance relative to target node
   * @param key Node to calculate distance against
   * @tparam C Contact type
   */
  def relativeOrder[C](key: Key): Order[Node[C]] = new Order[Node[C]] {
    private val order = Key.relativeOrder(key)

    override def compare(x: Node[C], y: Node[C]): Int = order.compare(x.key, y.key)
  }

  def relativeOrdering[C](key: Key): Ordering[Node[C]] =
    relativeOrder(key).compare(_, _)
}