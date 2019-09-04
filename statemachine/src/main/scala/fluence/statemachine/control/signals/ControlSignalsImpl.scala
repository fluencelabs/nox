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

package fluence.statemachine.control.signals

import cats.Monad
import cats.effect.Resource
import cats.effect.concurrent.{Deferred, MVar, Ref}
import cats.instances.long._
import cats.syntax.apply._
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.log.Log
import fluence.statemachine.api.StateHash
import fluence.statemachine.api.signals.{BlockReceipt, DropPeer}
import fluence.statemachine.control.HasOrderedProperty._
import fluence.statemachine.control.HasOrderedProperty.syntax._
import fluence.statemachine.control.LastCachingQueue
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * Sink and source for control events
 *
 * @param dropPeersRef Holds a set of DropPeer events. NOTE: since Tendermint 0.30.0 Validator set updates must be unique by pub key.
 * @param stopRef Deferred holding stop signal, completed when the worker should stop
 * @param receiptQueue Holds a collection of receipts, updated by node, consumed by AbciHandler
 * @param hashQueue Holds a collection of vm hashes, updated by AbciHandler, consumed by node
 * @tparam F Effect
 */
class ControlSignalsImpl[F[_]: Monad: Log](
  private val dropPeersRef: MVar[F, Set[DropPeer]],
  private val stopRef: Deferred[F, Unit],
  // Using simple queue instead of LastCachingQueue because currently there are no retries on receipts
  private val receiptQueue: fs2.concurrent.Queue[F, BlockReceipt],
  // getVmHash may be retried by node, so using LastCachingQueue
  private val hashQueue: LastCachingQueue[F, StateHash, Long],
  private val lastStateHashRef: Ref[F, StateHash]
) extends ControlSignals[F] {

  private def traceBU(msg: String) = Log[F].trace(Console.YELLOW + "BUD: " + msg + Console.RESET)

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
  def enqueueReceipt(receipt: BlockReceipt): F[Unit] =
    traceBU(s"enqueueReceipt ${receipt.height}") *> receiptQueue.enqueue1(receipt)

  /**
   * Retrieves block receipt, async-ly blocks until there's a receipt with specified height
   */
  def getReceipt(height: Long): F[BlockReceipt] = {
    traceBU(s"getReceipt $height") *> receiptQueue.dequeueByBoundary(height)
  }

  /**
   * Adds vm hash to queue, so node can retrieve it for block manifest uploading
   */
  override def enqueueStateHash(height: Long, hash: ByteVector): F[Unit] =
    traceBU(s"enqueueVmHash $height") *> hashQueue.enqueue1(StateHash(height, hash)) *> lastStateHashRef.set(
      StateHash(height, hash)
    )

  /**
   * Retrieves a single vm hash from queue. Called by node on block manifest uploading.
   * Async-ly blocks until there's a vmHash with specified height
   */
  override def getStateHash(height: Long): F[StateHash] =
    traceBU(s"getVmHash $height") *> hashQueue.dequeue(height)

  override def lastStateHash: F[StateHash] =
    lastStateHashRef.get
}
