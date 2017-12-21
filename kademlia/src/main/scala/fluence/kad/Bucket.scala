/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

package fluence.kad

import cats.data.StateT
import cats.syntax.applicativeError._
import cats.syntax.eq._
import cats.syntax.applicative._
import cats.{ MonadError, Show }

import scala.collection.immutable.Queue
import scala.concurrent.duration.Duration
import scala.language.higherKinds

/**
 * Kademlia's K-Bucket
 * @param maxSize Max size of bucket, K in paper
 * @param nodes Queue of nodes; last one is the least recent seen
 * @tparam C Node contacts
 */
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

  /**
   * Checks if the bucket should be updated (hence blocked for update) with the fresh node
   * @param node Node
   * @param pingExpiresIn Duration to ignore updates for a node
   */
  def shouldUpdate(node: Node[C], pingExpiresIn: Duration): Boolean =
    find(node.key).fold(true)(n ⇒
      !pingExpiresIn.isFinite() ||
        java.time.Duration.between(n.lastSeen, node.lastSeen).toMillis >= pingExpiresIn.toMillis
    )
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
   * @return updated Bucket, and true if bucket was updated with this node, false if it wasn't
   */
  def update[F[_], C](node: Node[C], rpc: C ⇒ KademliaRpc[F, C], pingExpiresIn: Duration)(implicit ME: MonadError[F, Throwable]): StateT[F, Bucket[C], Boolean] = {
    StateT.get[F, Bucket[C]].flatMap { b ⇒
      b.find(node.key) match {
        case Some(c) ⇒
          // put contact on top
          StateT.set(b.copy(nodes = b.nodes.filterNot(_.key === c.key).enqueue(node))).map(_ ⇒ true)

        case None if b.isFull ⇒ // Bucket is full, so we should check if we can drop the last node

          // ping last, if pong, put last on top and drop contact, if not, drop last and put contact on top
          val (last, nodes) = b.nodes.dequeue

          // The last contact in the queue is the oldest
          // If it's still very fresh, drop incoming node without pings
          if (pingExpiresIn.isFinite() && java.time.Duration.between(last.lastSeen, node.lastSeen).toMillis <= pingExpiresIn.toMillis) {
            StateT.pure(false)
          } else {

            // Ping last contact.
            // If it responds, enqueue it and drop the new node, otherwise, drop it and enqueue new one
            StateT.lift(rpc(last.contact).ping().attempt).flatMap {
              case Left(_) ⇒
                StateT.set(b.copy(nodes = nodes.enqueue(node))).map(_ ⇒ true)
              case Right(updatedLastContact) ⇒
                StateT.set(b.copy(nodes = nodes.enqueue(updatedLastContact))).map(_ ⇒ false)
            }
          }

        case None ⇒
          // put contact on top
          StateT.set(b.copy(nodes = b.nodes.enqueue(node))).map(_ ⇒ true)
      }
    }
  }

  /**
   * Read ops are pure functions
   * @tparam C Node contacts
   */
  trait ReadOps[C] {
    /**
     * Returns current bucket state
     * @param bucketId Bucket id, 0 to [[Key.BitLength]]
     */
    def read(bucketId: Int): Bucket[C]

    /**
     * Returns current bucket state
     * @param distanceKey Distance to get leading zeros from
     */
    def read(distanceKey: Key): Bucket[C] =
      read(distanceKey.zerosPrefixLen)
  }

  /**
   * Write ops are stateful
   * @tparam F Effect
   * @tparam C Node contacts
   */
  trait WriteOps[F[_], C] extends ReadOps[C] {

    /**
     * Runs a mutation on bucket, blocks the bucket from writes until mutation is complete
     * @param bucketId Bucket ID
     * @param mod Mutation
     * @tparam T Return value
     */
    protected def run[T](bucketId: Int, mod: StateT[F, Bucket[C], T]): F[T]

    /**
     * Performs bucket update if necessary, blocking the bucket
     * @param bucketId Bucket ID
     * @param node Fresh node
     * @param rpc RPC caller for Kademlia functions
     * @param pingExpiresIn Duration for the ping to be considered relevant
     * @param ME Monad error instance for the effect
     * @return True if node is updated in a bucket, false otherwise
     */
    def update(bucketId: Int, node: Node[C], rpc: C ⇒ KademliaRpc[F, C], pingExpiresIn: Duration)(implicit ME: MonadError[F, Throwable]): F[Boolean] =
      if (read(bucketId).shouldUpdate(node, pingExpiresIn)) {
        run(bucketId, Bucket.update(node, rpc, pingExpiresIn))
      } else {
        false.pure[F]
      }

  }

}
