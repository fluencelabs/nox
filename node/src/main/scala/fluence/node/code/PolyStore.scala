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
import java.util.concurrent.Executors

import cats.Monad
import cats.syntax.applicativeError._
import cats.data.EitherT
import cats.effect.{ContextShift, LiftIO, Sync, Timer}
import fluence.effects.castore.{ContentAddressableStore, StorageToFileFailed, StoreError}
import fluence.node.eth.state.StorageRef
import fluence.node.eth.state.StorageType.StorageType
import scodec.bits.ByteVector

import scala.concurrent.ExecutionContext
import scala.language.higherKinds

class PolyStore[F[_]: Sync: ContextShift](
  selector: StorageType => ContentAddressableStore[F],
  blockingCtx: ExecutionContext = ExecutionContext.fromExecutorService(Executors.newCachedThreadPool())
) {

  /**
   * Fetches contents and stores it into the dest path.
   * We assume that it's the file fetched.
   *
   * @param ref Content's hash and storage type
   * @param dest Destination file (not folder!). File will be created if it doesn't exist
   */
  def fetchTo(ref: StorageRef, dest: Path): EitherT[F, StoreError, Unit] = {
    selector(ref.storageType)
      .fetch(ref.storageHash)
      .flatMap(
        _.flatMap(bb ⇒ fs2.Stream.chunk(fs2.Chunk.byteBuffer(bb)))
          .through(fs2.io.file.writeAll(dest, blockingCtx))
          .compile
          .drain
          .attemptT
          .leftMap(err ⇒ StorageToFileFailed(ref.storageHash, dest, err).asInstanceOf[StoreError])
      )
  }

  def ls(ref: StorageRef): EitherT[F, StoreError, List[ByteVector]] = {
    selector(ref.storageType).ls(ref.storageHash)
  }

}
