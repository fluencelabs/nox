package fluence.kad

import cats.Order

case class Contact(key: Key)

object Contact {
  def relativeOrder(key: Key): Order[Contact] = new Order[Contact] {
    private val order = Key.relativeOrder(key)

    override def compare(x: Contact, y: Contact): Int = order.compare(x.key, y.key)
  }

  def relativeOrdering(key: Key): Ordering[Contact] =
    relativeOrder(key).compare(_, _)
}