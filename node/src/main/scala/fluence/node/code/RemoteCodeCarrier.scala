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

import java.nio.file.{Files, Path}

import cats.Monad
import cats.effect.{IO, LiftIO, Timer}
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.effects.Backoff
import fluence.effects.castore.StoreError
import fluence.node.eth.state.StorageRef

import scala.language.higherKinds

class RemoteCodeCarrier[F[_]: Timer: LiftIO: Monad](store: PolyStore[F])(
  implicit backoff: Backoff[StoreError]
) extends CodeCarrier[F] {

  /**
   * Downloads file from a storage, and stores it on a disk at the specified location.
   *
   * @param ref Reference to a code in a content addressable storage
   * @param filePath A path to store code at
   */
  private def downloadToFile(ref: StorageRef, filePath: Path): F[Unit] =
    backoff(store.fetchTo(ref, filePath))

  /**
   * If code doesn't exist yet, downloads it from a storage, and puts to `workerPath / vmcode / <hash>.wasm`
   *
   * @param workerPath a path to worker's directory
   * @param ref Reference to a code in a content addressable storage
   * @return A path where code is stored (currently it's `workerPath / vmcode / <hash>.wasm`)
   */
  private def downloadAndWriteCodeToFile(
    workerPath: Path,
    ref: StorageRef
  ): F[Path] =
    // TODO handle fail system errors properly?
    for {
      // TODO move vmcode to a config file
      dirPath ← IO(workerPath.resolve("vmcode")).to[F]
      _ <- IO(if (!dirPath.toFile.exists()) Files.createDirectory(dirPath)).to[F]

      //TODO check if file's Swarm hash corresponds to the address
      filePath <- IO(dirPath.resolve(ref.storageHash.toHex + ".wasm")).to[F]
      exists <- IO(filePath.toFile.exists()).to[F]
      _ <- if (exists) IO.unit.to[F]
      else
        for {
          tmpFile <- IO(Files.createTempFile("code_", "_wasm")).to[F]
          _ <- downloadToFile(ref, tmpFile)
          _ <- IO(Files.move(tmpFile, filePath)).to[F]
        } yield ()

    } yield dirPath

  /**
   * Downloads code from a storage, and writes it to `workerPath / vmcode / <hash>.wasm`
   *
   * @param ref a path to a code from the smart contract
   * @param workerPath a path to a worker's working directory
   * @return
   */
  override def carryCode(ref: StorageRef, workerPath: Path): F[Path] =
    downloadAndWriteCodeToFile(workerPath, ref)

}
