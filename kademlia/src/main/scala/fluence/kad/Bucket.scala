package fluence.kad

import cats.data.StateT
import cats.syntax.applicativeError._
import cats.syntax.eq._
import cats.{MonadError, Show}

import scala.collection.immutable.Queue
import scala.concurrent.duration.Duration
import scala.language.higherKinds

case class Bucket[C](maxSize: Int, nodes: Queue[Node[C]] = Queue.empty) {
  lazy val isFull: Boolean = nodes.lengthCompare(maxSize) >= 0

  lazy val size: Int = nodes.size

  def isEmpty: Boolean = nodes.isEmpty

  def nonEmpty: Boolean = nodes.nonEmpty

  /**
    * Lookups the bucket for a particular key, not making any external request
    *
    * @param key Contact key
    * @return optional found contact
    */
  def find(key: Key): Option[Node[C]] =
    nodes.find(_.key === key)

  /**
    * Generates a stream of inner nodes, sorted with given ordering
    * @param o Ordering to sort nodes with
    */
  def stream(implicit o: Ordering[Node[C]]): Stream[Node[C]] =
    nodes.sorted.toStream
}

object Bucket {
  implicit def show[C](implicit cs: Show[Node[C]]): Show[Bucket[C]] =
    b ⇒
      if (b.nodes.isEmpty) "[empty bucket]"
      else b.nodes.map(cs.show).mkString(s"[${b.size} of ${b.maxSize}\n\t", "\n\t", "]")

  /**
   * Performs bucket update.
   *
   * Whenever a node receives a communication from another, it updates the corresponding bucket.
   * If the contact already exists, it is '''moved''' to the ''head'' of the bucket.
   * Otherwise, if the bucket is not full, the new contact is '''added''' at the ''head''.
   * If the bucket is full, the node pings the contact at the ''end'' of the bucket's list.
   * If that least recently seen contact fails to respond in an ''unspecified'' reasonable time,
   * it is '''dropped''' from the list, and the new contact is added at the ''head''.
   * Otherwise the new contact is '''ignored''' for bucket updating purposes.
   *
   * @param node Contact to check and update
   * @param rpc    Ping function
   * @param ME      Monad error for StateT effect
   * @tparam F StateT effect
   * @return updated Bucket
   */
  def update[F[_], C](node: Node[C], rpc: C ⇒ KademliaRPC[F, C], pingTimeout: Duration)(implicit ME: MonadError[F, Throwable]): StateT[F, Bucket[C], Boolean] = {
    StateT.get[F, Bucket[C]].flatMap { b ⇒
      b.find(node.key) match {
        case Some(c) ⇒
          // put contact on top
          StateT.set(b.copy(nodes = b.nodes.filterNot(_.key === c.key).enqueue(node))).map(_ => true)

        case None if b.isFull ⇒ // Bucket is full, so we should check if we can drop the last node

          // ping last, if pong, put last on top and drop contact, if not, drop last and put contact on top
          val (last, nodes) = b.nodes.dequeue

          // The last contact in the queue is the oldest
          // If it's still very fresh, drop incoming node without pings
          if (pingTimeout.isFinite() && java.time.Duration.between(last.lastSeen, node.lastSeen).toMillis >= pingTimeout.toMillis) {
            StateT.pure(false)
          } else {

            // Ping last contact.
            // If it responds, enqueue it and drop the new node, otherwise, drop it and enqueue new one
            StateT.lift(rpc(last.contact).ping().attempt).flatMap {
              case Left(_) ⇒
                StateT.set(b.copy(nodes = nodes.enqueue(node))).map(_ => true)
              case Right(updatedLastContact) ⇒
                StateT.set(b.copy(nodes = nodes.enqueue(updatedLastContact))).map(_ => false)
            }
          }

        case None ⇒
          // put contact on top
          StateT.set(b.copy(nodes = b.nodes.enqueue(node))).map(_ => true)
      }
    }
  }

  trait ReadOps[C] {
    def read(bucketId: Int): Bucket[C]

    def read(distanceKey: Key): Bucket[C] =
      read(distanceKey.zerosPrefixLen)
  }

  trait WriteOps[F[_], C] extends ReadOps[C] {

    protected def run[T](bucketId: Int, mod: StateT[F, Bucket[C], T]): F[T]

    def update(bucketId: Int, node: Node[C], rpc: C ⇒ KademliaRPC[F, C], pingTimeout: Duration)
              (implicit ME: MonadError[F, Throwable]): F[Boolean] =
      run(bucketId, Bucket.update(node, rpc, pingTimeout))

  }

}