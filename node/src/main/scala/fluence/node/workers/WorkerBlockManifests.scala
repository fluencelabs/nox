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

package fluence.node.workers

import cats.effect.concurrent.Ref
import cats.effect.{ContextShift, LiftIO, Resource, Sync}
import cats.Monad
import fluence.effects.receipt.storage.ReceiptStorage
import fluence.effects.tendermint.block.history.{BlockManifest, Receipt}
import fluence.log.Log

import scala.language.higherKinds

/**
 * Service for Worker's block manifests storage, replication, and retrieval
 *
 * @param receiptStorage Receipt storage
 * @param lastManifestRef Last stored manifest, could be empty
 */
class WorkerBlockManifests[F[_]: Monad](
  val receiptStorage: ReceiptStorage[F],
  lastManifestRef: Ref[F, Option[BlockManifest]]
) {

  /**
   * Returns the last known manifest, if any
   */
  def lastManifestOpt: F[Option[BlockManifest]] =
    lastManifestRef.get

  /**
   * Updates the last known manifest
   * TODO broadcast manifest to Kademlia
   *
   * @param manifest Block manifest
   * @param receipt This block manifest's receipt
   */
  def onUploaded(manifest: BlockManifest, receipt: Receipt): F[Unit] =
    lastManifestRef.set(Some(manifest))
}

object WorkerBlockManifests {

  /**
   * Makes new WorkerBlockManifests
   *
   * @param receiptStorageResource Makes a new ReceiptStorage for this app
   */
  def make[F[_]: Sync: LiftIO: Log: ContextShift](
    receiptStorageResource: Resource[F, ReceiptStorage[F]]
  ): Resource[F, WorkerBlockManifests[F]] =
    for {
      receiptStorage ← receiptStorageResource
      lastManifestRef ← Resource.liftF(Ref.of(Option.empty[BlockManifest]))
    } yield new WorkerBlockManifests[F](receiptStorage, lastManifestRef)
}
