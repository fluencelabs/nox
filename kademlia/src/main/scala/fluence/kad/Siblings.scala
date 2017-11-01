package fluence.kad

import cats.{ Applicative, Show }
import cats.data.StateT
import cats.syntax.eq._
import cats.syntax.functor._

import scala.collection.SortedSet
import scala.language.higherKinds

/**
 * List of the closest known nodes for the current one
 * @param nodes Nodes, ordered by distance
 * @param maxSize Maximum number of sibling nodes to keep
 * @tparam C Contact type
 */
case class Siblings[C] private (nodes: SortedSet[Node[C]], maxSize: Int) {

  lazy val isFull: Boolean = nodes.size >= maxSize

  lazy val size: Int = nodes.size

  def isEmpty: Boolean = nodes.isEmpty

  def nonEmpty: Boolean = nodes.nonEmpty

  def find(key: Key): Option[Node[C]] = nodes.find(_.key === key)

  def contains(key: Key): Boolean = nodes.exists(_.key === key)

  def add(node: Node[C]): Siblings[C] =
    copy((nodes + node).take(maxSize))
}

object Siblings {
  implicit def show[C](implicit ks: Show[Key]): Show[Siblings[C]] =
    s ⇒ s.nodes.toSeq.map(_.key).map(ks.show).mkString(s"\nSiblings: ${s.size}\n\t", "\n\t", "")

  /**
   * Builds a Siblings instance with ordering relative to nodeId
   * @param nodeId Current node's id
   * @param maxSize Maximum number of sibling nodes to keep
   * @tparam C Contact info
   */
  def apply[C](nodeId: Key, maxSize: Int): Siblings[C] = {
    implicit val ordering: Ordering[Node[C]] = Node.relativeOrdering(nodeId)
    new Siblings[C](SortedSet.empty, maxSize)
  }

  /**
   * Read operations over current Siblings state
   * @tparam C Contact
   */
  trait ReadOps[C] {
    def read: Siblings[C]
  }

  /**
   * Stateful Write operations for Siblings
   * @tparam F Effect type
   * @tparam C Contact
   */
  trait WriteOps[F[_], C] extends ReadOps[C] {
    protected def run[T](mod: StateT[F, Siblings[C], T]): F[T]

    /**
     * Adds a node
     * @param node Node to add
     * @param F Effect
     * @return True if node is in Siblings after update
     */
    def add(node: Node[C])(implicit F: Applicative[F]): F[Boolean] =
      run(StateT.modify(_.add(node))).map(_ ⇒ read.contains(node.key))
  }
}