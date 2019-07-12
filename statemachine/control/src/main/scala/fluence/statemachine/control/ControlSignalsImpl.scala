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

import cats.FlatMap
import cats.effect.Resource
import cats.effect.concurrent.{Deferred, MVar}
import cats.syntax.flatMap._
import cats.syntax.functor._
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * Sink and source for control events
 *
 * @param dropPeersRef Holds a set of DropPeer events. NOTE: since Tendermint 0.30.0 Validator set updates must be unique by pub key.
 * @param stopRef Deferred holding stop signal, completed when the worker should stop
 * @tparam F Effect
 */
class ControlSignalsImpl[F[_]: FlatMap](
  private val dropPeersRef: MVar[F, Set[DropPeer]],
  private val stopRef: Deferred[F, Unit],
  private val receiptRef: MVar[F, BlockReceipt],
  private val hashRef: MVar[F, ByteVector]
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
  def putReceipt(receipt: BlockReceipt): F[Unit] = receiptRef.put(receipt)

  /**
   * Retrieves block receipt, async blocks until there's a receipt
   */
  val receipt: F[BlockReceipt] = receiptRef.take

  /**
   * Stores vm hash to memory, so node can retrieve it for block manifest uploading
   */
  def putVmHash(hash: ByteVector): F[Unit] = hashRef.put(hash)

  def setVmHash(hash: ByteVector): F[Unit] = hashRef.tryTake >> hashRef.put(hash)

  /**
   * Retrieves stored vm hash. Called by node on block manifest uploading
   */
  val vmHash: F[ByteVector] = hashRef.take
}
