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

import java.util.concurrent.TimeUnit

import cats.data.StateT
import cats.effect.{LiftIO, Timer}
import cats.syntax.eq._
import cats.syntax.applicative._
import cats.syntax.flatMap._
import cats.{Monad, Show}
import fluence.kad.protocol.{KademliaRpc, Key, Node}

import scala.collection.immutable.Queue
import scala.concurrent.duration.Duration
import scala.language.higherKinds

/**
 * Kademlia's K-Bucket
 *
 * @param maxSize Max size of bucket, K in paper
 * @param records Queue of records; last one is the least recent seen
 * @tparam C Node contacts
 */
case class Bucket[C](maxSize: Int, private val records: Queue[Bucket.Record[C]] = Queue.empty) {
  lazy val nodes: Seq[Node[C]] = records.map(_.node)

  lazy val isFull: Boolean = records.lengthCompare(maxSize) >= 0

  lazy val size: Int = records.size

  def isEmpty: Boolean = records.isEmpty

  def nonEmpty: Boolean = records.nonEmpty

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
   *
   * @param node Node
   * @param pingExpiresIn Duration to ignore updates for a node
   */
  def shouldUpdate(node: Node[C], pingExpiresIn: Duration, currentTime: Long): Boolean =
    records
      .find(_.node.key === node.key)
      .fold(true)(
        n ⇒ !pingExpiresIn.isFinite() || currentTime - n.lastSeenEpochMillis >= pingExpiresIn.toMillis
      )
}

object Bucket {

  /**
   * Record is an internal in-memory storage for a Node
   */
  private[kad] case class Record[C](node: Node[C], lastSeenEpochMillis: Long)

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
   * @tparam F StateT effect
   * @return updated Bucket, and true if bucket was updated with this node, false if it wasn't
   */
  def update[F[_]: LiftIO: Monad: Timer, C](
    node: Node[C],
    rpc: C ⇒ KademliaRpc[C],
    pingExpiresIn: Duration
  ): StateT[F, Bucket[C], Boolean] = {
    def mapRecords(f: Queue[Record[C]] ⇒ Queue[Record[C]]): StateT[F, Bucket[C], Unit] =
      StateT.modify[F, Bucket[C]](b ⇒ b.copy(records = f(b.records)))

    val time: StateT[F, Bucket[C], Long] = StateT.liftF(Timer[F].clockRealTime(TimeUnit.MILLISECONDS))

    def makeRecord(n: Node[C]): StateT[F, Bucket[C], Record[C]] =
      time.map(Record(n, _))

    StateT.get[F, Bucket[C]].flatMap[Boolean, Bucket[C]] { b ⇒
      b.find(node.key) match {
        case Some(c) ⇒
          // put contact on top
          for {
            r ← makeRecord(node)
            _ ← mapRecords(_.filterNot(_.node.key === c.key).enqueue(r))
          } yield true

        case None if b.isFull ⇒ // Bucket is full, so we should check if we can drop the last node

          // ping last, if pong, put last on top and drop contact, if not, drop last and put contact on top
          val (last, records) = b.records.dequeue

          def enqueue(n: Node[C], nodeInserted: Boolean): StateT[F, Bucket[C], Boolean] =
            for {
              r ← makeRecord(n)
              _ ← StateT.set(b.copy(records = records.enqueue(r)))
            } yield nodeInserted

          // The last contact in the queue is the oldest
          // If it's still very fresh, drop incoming node without pings
          time.map(_ - last.lastSeenEpochMillis <= pingExpiresIn.toMillis).flatMap {
            case false ⇒
              StateT.pure(false)
            case true ⇒
              // Ping last contact.
              // If it responds, enqueue it and drop the new node, otherwise, drop it and enqueue new one
              StateT.liftF(rpc(last.node.contact).ping().attempt.to[F]).flatMap {
                case Left(_) ⇒
                  enqueue(node, nodeInserted = true)
                case Right(updatedLastContact) ⇒
                  enqueue(updatedLastContact, nodeInserted = false)
              }
          }

        case None ⇒
          // put contact on top
          for {
            r ← makeRecord(node)
            _ ← mapRecords(_.enqueue(r))
          } yield true
      }
    }
  }

  /**
   * Read ops are pure functions
   *
   * @tparam C Node contacts
   */
  trait ReadOps[C] {

    /**
     * Returns current bucket state
     *
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
   *
   * @tparam F Effect
   * @tparam C Node contacts
   */
  trait WriteOps[F[_], C] extends ReadOps[C] {

    /**
     * Runs a mutation on bucket, blocks the bucket from writes until mutation is complete
     *
     * @param bucketId Bucket ID
     * @param mod Mutation
     * @tparam T Return value
     */
    protected def run[T](bucketId: Int, mod: StateT[F, Bucket[C], T]): F[T]

    /**
     * Performs bucket update if necessary, blocking the bucket
     *
     * @param bucketId Bucket ID
     * @param node Fresh node
     * @param rpc RPC caller for Kademlia functions
     * @param pingExpiresIn Duration for the ping to be considered relevant
     * @return True if node is updated in a bucket, false otherwise
     */
    def update(bucketId: Int, node: Node[C], rpc: C ⇒ KademliaRpc[C], pingExpiresIn: Duration)(
      implicit liftIO: LiftIO[F],
      F: Monad[F],
      timer: Timer[F]
    ): F[Boolean] =
      timer.clockRealTime(TimeUnit.MILLISECONDS).flatMap { time ⇒
        if (read(bucketId).shouldUpdate(node, pingExpiresIn, time)) {
          run(bucketId, Bucket.update(node, rpc, pingExpiresIn))
        } else
          false.pure[F]
      }
  }

}
