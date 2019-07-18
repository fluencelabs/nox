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

package fluence.statemachine.control

import cats.effect.concurrent.{Deferred, MVar}
import cats.effect.{Resource, Sync}
import cats.syntax.applicative._
import cats.syntax.either._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.{FlatMap, Monad}
import fluence.statemachine.control.HasHeight.syntax._
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * Sink and source for control events
 *
 * @param dropPeersRef Holds a set of DropPeer events. NOTE: since Tendermint 0.30.0 Validator set updates must be unique by pub key.
 * @param stopRef Deferred holding stop signal, completed when the worker should stop
 * @tparam F Effect
 */
class ControlSignalsImpl[F[_]: FlatMap: Sync](
  private val dropPeersRef: MVar[F, Set[DropPeer]],
  private val stopRef: Deferred[F, Unit],
  private val receiptQueue: fs2.concurrent.Queue[F, BlockReceipt],
  private val hashQueue: fs2.concurrent.Queue[F, VmHash]
) extends ControlSignals[F] {

  /**
   * Add a new DropPeer event
   */
  def dropPeer(drop: DropPeer): F[Unit] =
    for {
      changes <- dropPeersRef.take
      _ <- dropPeersRef.put(changes + drop)
    } yield ()

  /**
   * Move list of current DropPeer events from ControlSignals to call-site
   * dropPeersRef is emptied on resource's acquisition, and filled with Nil after resource is used
   * Using Resource this way guarantees exclusive access to data
   *
   * @return Resource with List of DropPeer signals
   */
  val dropPeers: Resource[F, Set[DropPeer]] =
    Resource.make(dropPeersRef.tryTake.map(_.getOrElse(Set.empty)))(_ => dropPeersRef.tryPut(Set.empty).void)

  /**
   * Orders the worker to stop
   */
  def stopWorker(): F[Unit] = stopRef.complete(())

  /**
   * Will evaluate once the worker should stop
   */
  val stop: F[Unit] = stopRef.get

  /**
   * Stores block receipt in memory, async blocks if previous receipt is still there
   * Receipt comes from node through control rpc
   *
   * @param receipt Receipt to store
   */
  def enqueueReceipt(receipt: BlockReceipt): F[Unit] = receiptQueue.enqueue1(receipt)

  /**
   * Retrieves block receipt, async blocks until there's a receipt
   */
  def getReceipt(height: Long): F[BlockReceipt] = dequeueByHeight(receiptQueue, height)

  /**
   * Adds vm hash to queue, so node can retrieve it for block manifest uploading
   */
  override def enqueueVmHash(height: Long, hash: ByteVector): F[Unit] = hashQueue.enqueue1(VmHash(height, hash))

  /**
   * Retrieves a single vm hash from queue. Called by node on block manifest uploading
   */
  override def getVmHash(height: Long): F[VmHash] =
    // Filter here because after blocks replay (on restart) there would be extraneous vm hashes for empty blocks
    dequeueByHeight(hashQueue, height)

  private def dequeueByHeight[A: HasHeight](queue: fs2.concurrent.Queue[F, A], height: Long): F[A] =
    Monad[F].tailRecM(queue) { q =>
      q.dequeue1.flatMap { elem =>
        if (elem.height < height) {
          // keep looking
          q.asLeft[A].pure[F]
        } else if (elem.height > height) {
          // corner case: elements aren't in order, try to reorder them
          q.enqueue1(elem).as(q.asLeft)
        } else {
          // got it!
          elem.asRight[fs2.concurrent.Queue[F, A]].pure[F]
        }
      }
    }
}
