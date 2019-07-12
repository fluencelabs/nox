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
import cats.syntax.applicativeError._
import cats.syntax.flatMap._
import cats.syntax.functor._
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
   * Stores block receipt in memory, async blocks if previous receipt is still there
   * Receipt comes from node through control rpc
   *
   * @param receipt Receipt to store
   */
  // TODO: move that method to a separate interface
  def putReceipt(receipt: BlockReceipt): F[Unit]

  /**
   * Retrieves block receipt, async blocks until there's a receipt
   */
  val receipt: F[BlockReceipt]

  /**
   * Stores vm hash to memory, so node can retrieve it for block manifest uploading
   */
  def putVmHash(hash: ByteVector): F[Unit]

  def setVmHash(hash: ByteVector): F[Unit]

  /**
   * Retrieves stored vm hash. Called by node on block manifest uploading
   */
  // TODO: move that method to a separate interface
  val vmHash: F[ByteVector]
}

object ControlSignals {

  /**
   * Create a resource holding ControlSignals. Stop ControlSignals after resource is used.
   *
   * @tparam F Effect
   * @return Resource holding a ControlSignals instance
   */
  def apply[F[_]: Concurrent](): Resource[F, ControlSignals[F]] =
    Resource.make(
      for {
        dropPeersRef ← MVar[F].of[Set[DropPeer]](Set.empty)
        stopRef ← Deferred[F, Unit]
        receiptRef <- MVar[F].empty[BlockReceipt]
        hashRef <- MVar[F].empty[ByteVector]
        instance = new ControlSignalsImpl[F](dropPeersRef, stopRef, receiptRef, hashRef)
      } yield instance: ControlSignals[F]
    ) { s =>
      Sync[F].suspend(
        // Tell worker to stop if ControlSignals is dropped
        s.stopWorker().attempt.void
      )
    }
}
