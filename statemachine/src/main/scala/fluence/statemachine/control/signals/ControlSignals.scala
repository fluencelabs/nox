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

import cats.effect.concurrent.{Deferred, MVar, Ref}
import cats.effect.{Concurrent, Resource, Sync}
import cats.instances.long._
import cats.syntax.applicativeError._
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.log.Log
import fluence.statemachine.api.StateHash
import fluence.statemachine.api.signals.{BlockReceipt, DropPeer}
import fluence.statemachine.control.LastCachingQueue
import scodec.bits.ByteVector

import scala.language.higherKinds

trait ControlSignals[F[_]] {

  /**
   * Add a new DropPeer event
   */
  // TODO: move that method to a separate interface
  def dropPeer(drop: DropPeer): F[Unit]

  /**
   * Move list of current DropPeer events from ControlSignals to call-site
   * dropPeersRef is emptied on resource's acquisition, and filled with Nil after resource is used
   * Using Resource this way guarantees exclusive access to data
   *
   * @return Resource with List of DropPeer signals
   */
  val dropPeers: Resource[F, Set[DropPeer]]

  /**
   * Orders the worker to stop
   */
  // TODO: move that method to a separate interface
  def stopWorker(): F[Unit]

  /**
   * Will evaluate once the worker should stop
   */
  val stop: F[Unit]

  /**
   * Puts block receipt to a queue
   * Receipt comes from node through control rpc
   *
   * @param receipt Receipt to store
   */
  // TODO: move that method to a separate interface
  def enqueueReceipt(receipt: BlockReceipt): F[Unit]

  /**
   * Retrieves block receipt for a given height, async-ly blocks until there's a receipt
   */
  def getReceipt(height: Long): F[BlockReceipt]

  /**
   * Adds vm hash to queue, so node can retrieve it for block manifest uploading
   */
  def enqueueStateHash(height: Long, hash: ByteVector): F[Unit]

  /**
   * Retrieves a single vm hash from queue. Called by node on block manifest uploading
   */
  // TODO: move that method to a separate interface
  def getStateHash(height: Long): F[StateHash]

  def lastStateHash: F[StateHash]
}

object ControlSignals {

  /**
   * Create a resource holding ControlSignals. Stop ControlSignals after resource is used.
   *
   * @tparam F Effect
   * @return Resource holding a ControlSignals instance
   */
  def apply[F[_]: Concurrent: Log](): Resource[F, ControlSignals[F]] =
    Resource.make(
      for {
        dropPeersRef ← MVar[F].of[Set[DropPeer]](Set.empty)
        stopRef ← Deferred[F, Unit]
        // getVmHash may be retried by node, so using LastCachingQueue
        hashQueue <- LastCachingQueue[F, StateHash, Long]
        // Using simple queue instead of LastCachingQueue because currently there are no retries on receipts
        receiptQueue <- fs2.concurrent.Queue.unbounded[F, BlockReceipt]

        lastHashRef ← Ref.of[F, StateHash](StateHash(0, ByteVector.empty))

        instance = new ControlSignalsImpl[F](dropPeersRef, stopRef, receiptQueue, hashQueue, lastHashRef)
      } yield instance: ControlSignals[F]
    ) { s =>
      Sync[F].suspend(
        // Tell worker to stop if ControlSignals is dropped
        s.stopWorker().attempt.void
      )
    }
}
