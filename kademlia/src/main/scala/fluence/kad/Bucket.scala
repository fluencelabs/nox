package fluence.kad

import cats.{ Applicative, MonadError, Show }
import cats.data.StateT
import cats.syntax.eq._
import cats.syntax.flatMap._
import cats.syntax.applicativeError._
import cats.syntax.applicative._

import scala.collection.immutable.Queue
import scala.concurrent.duration.Duration
import scala.language.higherKinds

case class Bucket[C](maxSize: Int, nodes: Queue[Node[C]] = Queue.empty) {
  lazy val isFull: Boolean = nodes.lengthCompare(maxSize) >= 0

  lazy val size: Int = nodes.size

  def isEmpty: Boolean = nodes.isEmpty

  def nonEmpty: Boolean = nodes.nonEmpty
}

object Bucket {
  implicit def show[C](implicit cs: Show[Node[C]]): Show[Bucket[C]] =
    b ⇒
      if (b.nodes.isEmpty) "[empty bucket]"
      else b.nodes.map(cs.show).mkString(s"[${b.size} of ${b.maxSize}\n\t", "\n\t", "]")

  /**
   * Returns the bucket state
   *
   * @tparam F StateT effect
   * @return The bucket
   */
  def bucket[F[_]: Applicative, C]: StateT[F, Bucket[C], Bucket[C]] =
    StateT.get

  /**
   * Lookups the bucket for a particular key, not making any external request
   *
   * @param key Contact key
   * @tparam F StateT effect
   * @return optional found contact
   */
  def find[F[_]: Applicative, C](key: Key): StateT[F, Bucket[C], Option[Node[C]]] =
    bucket[F, C].map { b ⇒
      b.nodes.find(_.key === key)
    }

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
  def update[F[_], C](node: Node[C], rpc: C ⇒ KademliaRPC[F, C], pingTimeout: Duration)(implicit ME: MonadError[F, Throwable]): StateT[F, Bucket[C], Unit] = {
    bucket[F, C].flatMap { b ⇒
      find[F, C](node.key).flatMap {
        case Some(c) ⇒
          // put contact on top
          StateT set b.copy(nodes = b.nodes.filterNot(_.key === c.key).enqueue(node))

        case None if b.isFull ⇒ // Bucket is full, so we should check if we can drop the last node

          // ping last, if pong, put last on top and drop contact, if not, drop last and put contact on top
          val (last, nodes) = b.nodes.dequeue

          // The last contact in the queue is the oldest
          // If it's still very fresh, drop incoming node without pings
          if (pingTimeout.isFinite() && java.time.Duration.between(last.lastSeen, node.lastSeen).toMillis >= pingTimeout.toMillis) {
            StateT.pure(())
          } else {

            // Ping last contact.
            // If it responds, enqueue it and drop the new node, otherwise, drop it and enqueue new one
            StateT setF rpc(last.contact).ping().attempt.flatMap {
              case Left(_) ⇒
                b.copy(nodes = nodes.enqueue(node)).pure
              case Right(updatedLastContact) ⇒
                b.copy(nodes = nodes.enqueue(updatedLastContact)).pure
            }
          }

        case None ⇒
          // put contact on top
          StateT set b.copy(nodes = b.nodes.enqueue(node))
      }
    }
  }

}