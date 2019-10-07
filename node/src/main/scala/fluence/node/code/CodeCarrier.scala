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

package fluence.node.code

import java.nio.file.Path

import cats.effect.{Concurrent, ContextShift, LiftIO, Sync, Timer}
import fluence.effects.Backoff
import fluence.effects.castore.StoreError
import fluence.effects.ipfs.IpfsStore
import fluence.effects.sttp.SttpStreamEffect
import fluence.effects.swarm.SwarmStore
import fluence.log.Log
import fluence.node.config.storage.RemoteStorageConfig
import fluence.worker.eth.{StorageRef, StorageType}

import scala.language.higherKinds

trait CodeCarrier[F[_]] {

  /**
   * Transfers (carries) the code from a storage to the directory specified by storagePath
   *
   * @param ref A reference to a code in a storage
   * @param storagePath A path where to put the code
   * @return Path to the copied code inside storagePath
   */
  def carryCode(ref: StorageRef, storagePath: Path)(implicit log: Log[F]): F[Path]
}

object CodeCarrier {

  def apply[F[_]: Sync: ContextShift: Concurrent: Timer: LiftIO: SttpStreamEffect](
    config: RemoteStorageConfig
  ): CodeCarrier[F] =
    if (config.enabled) {
      implicit val b: Backoff[StoreError] = Backoff.default
      val swarmStore = SwarmStore[F](config.swarm.address, config.swarm.readTimeout)
      val ipfsStore = IpfsStore[F](config.ipfs.address, config.ipfs.readTimeout)
      val polyStore = new PolyStore[F]({
        case StorageType.Swarm => swarmStore
        case StorageType.Ipfs  => ipfsStore
      })
      new RemoteCodeCarrier[F](polyStore)
    } else
      new LocalCodeCarrier[F]()
}
