/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package fluence.kad.state

import java.util.concurrent.TimeUnit

import cats.data.StateT
import cats.effect.{Clock, LiftIO}
import cats.syntax.eq._
import cats.syntax.functor._
import cats.syntax.apply._
import cats.{Monad, Show}
import fluence.kad.protocol.{KademliaRpc, Key, Node}
import fluence.log.Log

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
    nodes.sorted(o).toStream

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
   * @return updated Bucket, and Left if this node wasn't saved, Right with optional dropped node if it was
   */
  def update[F[_]: LiftIO: Monad: Clock: Log, C](
    node: Node[C],
    rpc: C ⇒ KademliaRpc[C],
    pingExpiresIn: Duration
  ): StateT[F, Bucket[C], ModResult[C]] = {
    def mapRecords(f: Queue[Record[C]] ⇒ Queue[Record[C]]): StateT[F, Bucket[C], Unit] =
      StateT.modify[F, Bucket[C]](b ⇒ b.copy(records = f(b.records)))

    val time: StateT[F, Bucket[C], Long] = StateT.liftF(Clock[F].realTime(TimeUnit.MILLISECONDS))

    def makeRecord(n: Node[C]): StateT[F, Bucket[C], Record[C]] =
      time.map(Record(n, _))

    // Log for StateT[F, Bucket[C], ?]
    val log = Log.stateT[F, Bucket[C]]

    StateT.get[F, Bucket[C]].flatMap[ModResult[C], Bucket[C]] { b ⇒
      b.find(node.key) match {
        case Some(c) ⇒
          // put contact on top
          for {
            r ← makeRecord(node)
            _ ← mapRecords(_.filterNot(_.node.key === c.key).enqueue(r))
            _ ← log.trace(s"Bucket updated ${node.key}: was present, moved on top")
          } yield ModResult.updated(node)

        case None if b.isFull ⇒ // Bucket is full, so we should check if we can drop the last node

          // ping last, if pong, put last on top and drop contact, if not, drop last and put contact on top
          val (last, records) = b.records.dequeue

          def enqueue(n: Node[C]): StateT[F, Bucket[C], Unit] =
            for {
              r ← makeRecord(n)
              _ ← StateT.set(b.copy(records = records.enqueue(r)))
            } yield ()

          // The last contact in the queue is the oldest
          // If it's still very fresh, drop incoming node without pings
          time.map(t ⇒ !pingExpiresIn.isFinite() || t - last.lastSeenEpochMillis <= pingExpiresIn.toMillis).flatMap {
            case false ⇒
              StateT.pure(ModResult.noop)
            case true ⇒
              // Ping last contact.
              // If it responds, enqueue it and drop the new node, otherwise, drop it and enqueue new one
              StateT.liftF(rpc(last.node.contact).ping().attempt.to[F]).flatMap {
                case Left(_) ⇒
                  for {
                    _ ← enqueue(node)
                    _ ← log.trace(s"Bucket updated ${node.key}")
                    _ ← log.trace(s"Bucket removed ${last.node.key}: ping failed")
                  } yield ModResult.updated(node).remove(last.node.key)

                case Right(updatedLastContact) ⇒
                  enqueue(updatedLastContact) *>
                    log.trace(s"Node updated last contact: ${updatedLastContact.key}; offered ${node.key} dropped") as
                    ModResult.updated(updatedLastContact)
              }
          }

        case None ⇒
          // put contact on top
          for {
            r ← makeRecord(node)
            _ ← mapRecords(_.enqueue(r))
            _ ← log.trace(s"Bucket added ${node.key}")
          } yield ModResult.updated(node)
      }
    }
  }

  /**
   * Removes a node from the bucket by node's key
   *
   * @tparam F Monad
   * @tparam C Contact type
   * @param key Key to remove
   * @return Modification result
   */
  def remove[F[_]: Monad: Log, C](key: Key): StateT[F, Bucket[C], ModResult[C]] =
    StateT.get[F, Bucket[C]].map(_.find(key)).flatMap {
      case None ⇒
        StateT.pure(ModResult.noop)

      case _ ⇒
        StateT
          .modify[F, Bucket[C]](
            bucket ⇒
              bucket.copy(
                records = bucket.records.filterNot(_.node.key === key)
            )
          ) *>
          Log.stateT[F, Bucket[C]].trace(s"Bucket removed $key") as
          ModResult.removed(key)
    }

}
