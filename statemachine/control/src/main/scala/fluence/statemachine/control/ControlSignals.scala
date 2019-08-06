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
import cats.effect.{Concurrent, Resource, Sync}
import cats.syntax.flatMap._
import fluence.log.Log
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
  def enqueueVmHash(height: Long, hash: ByteVector): F[Unit]

  /**
   * Retrieves a single vm hash from queue. Called by node on block manifest uploading
   */
  // TODO: move that method to a separate interface
  def getVmHash(height: Long): F[VmHash]
}

object ControlSignals {
  import cats.implicits._

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
        hashQueue <- LastCachingQueue[F, VmHash, Long]
        receiptQueue <- LastCachingQueue[F, BlockReceipt, Long]
        instance = new ControlSignalsImpl[F](dropPeersRef, stopRef, receiptQueue, hashQueue)
      } yield instance: ControlSignals[F]
    ) { s =>
      Sync[F].suspend(
        // Tell worker to stop if ControlSignals is dropped
        s.stopWorker().attempt.void
      )
    }
}
