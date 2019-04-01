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

package fluence.effects.castore

import java.nio.ByteBuffer
import java.nio.file.Path
import java.util.concurrent.Executors

import cats.syntax.applicativeError._
import cats.Monad
import cats.data.EitherT
import cats.effect.{Concurrent, ContextShift}
import scodec.bits.ByteVector

import scala.concurrent.ExecutionContext
import scala.language.higherKinds

abstract class ContentAddressableStore[F[_]: Concurrent: ContextShift](
  blockingCtx: ExecutionContext = ExecutionContext.fromExecutorService(Executors.newCachedThreadPool())
) {

  /**
   * Fetches contents corresponding to the given hash.
   * We assume that it's the file fetched.
   *
   * @param hash Content's hash
   */
  def fetch(hash: ByteVector): EitherT[F, StoreError, fs2.Stream[F, ByteBuffer]]

  /**
   * Fetches contents and stores it into the dest path.
   * We assume that it's the file fetched.
   *
   * @param hash Content's hash
   * @param dest Destination file (not folder!). File will be created if it doesn't exist
   */
  def fetchTo(hash: ByteVector, dest: Path): EitherT[F, StoreError, Unit] =
    fetch(hash).flatMap(
      _.flatMap(bb ⇒ fs2.Stream.chunk(fs2.Chunk.byteBuffer(bb)))
        .to(fs2.io.file.writeAll(dest, blockingCtx))
        .compile
        .drain
        .attemptT
        .leftMap(err ⇒ StorageToFileFailed(hash, dest, err))
    )
}
