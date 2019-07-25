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

import java.nio.file.Path

import cats.effect.concurrent.MVar
import cats.effect.{Concurrent, ContextShift, LiftIO, Resource}
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.{Applicative, Monad}
import fluence.effects.receipt.storage.{KVReceiptStorage, ReceiptStorage}
import fluence.effects.tendermint.block.history.{BlockManifest, Receipt}
import fluence.log.Log

import scala.language.higherKinds

/**
 * Worker's block manifests service
 *
 * @param receiptStorage Receipt storage
 * @param lastManifestRef Last stored manifest, could be empty
 */
class WorkerBlockManifests[F[_]: Monad](
  val receiptStorage: ReceiptStorage[F],
  lastManifestRef: MVar[F, BlockManifest]
) {

  /**
   * Returns the last known manifest, blocks until there's any
   */
  def lastManifest: F[BlockManifest] =
    lastManifestRef.read

  /**
   * Returns the last known manifest, if any
   */
  def lastManifestOpt: F[Option[BlockManifest]] =
    lastManifestRef.tryTake.flatMap {
      case r @ Some(m) ⇒ lastManifestRef.put(m) as r
      case None ⇒ Applicative[F].pure(None)
    }

  /**
   * Updates the last known manifest
   * TODO broadcast manifest to Kademlia
   *
   * @param manifest Block manifest
   * @param receipt This block manifest's receipt
   */
  def onUploaded(manifest: BlockManifest, receipt: Receipt): F[Unit] =
    lastManifestRef.tryTake >> lastManifestRef.put(manifest)
}

object WorkerBlockManifests {

  /**
   * Makes new WorkerBlockManifests
   *
   * @param appId Worker's application id
   * @param storageRoot Storage root, to be used with [[KVReceiptStorage.make]]
   */
  def make[F[_]: Concurrent: LiftIO: Log: ContextShift](appId: Long,
                                                        storageRoot: Path): Resource[F, WorkerBlockManifests[F]] =
    for {
      receiptStorage ← KVReceiptStorage.make[F](appId, storageRoot)
      lastManifestRef ← Resource.liftF(MVar.empty[F, BlockManifest])
    } yield new WorkerBlockManifests[F](receiptStorage, lastManifestRef)
}
