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
import fluence.effects.castore.{ContentAddressableStore, StoreError}
import scodec.bits.ByteVector

import scala.language.higherKinds

class RemoteCodeStore[F[_]: Timer: LiftIO: Monad](store: ContentAddressableStore[F])(
  implicit backoff: Backoff[StoreError]
) extends CodeStore[F] {

  /**
   * Downloads file from the Swarm and store it on a disk.
   * @param hash a code address and a Swarm URL address
   * @param filePath a path to code to store
   */
  private def downloadToFile(hash: ByteVector, filePath: Path): F[Unit] =
    backoff(store.fetchTo(hash, filePath))

  /**
   * Checks if there is no code already then download a file from the Swarm and store it to a disk.
   * @param workerPath a path to worker's directory
   * @param hash a code address and a Swarm URL address
   * @return a path to a code
   */
  private def downloadAndWriteCodeToFile(
    workerPath: Path,
    hash: ByteVector
  ): F[Path] =
    // TODO handle fail system errors properly?
    for {
      // TODO why to hardcode the directory and its creation?
      dirPath ← IO(workerPath.resolve("vmcode")).to[F]
      _ <- IO(if (!dirPath.toFile.exists()) Files.createDirectory(dirPath)).to[F]

      //TODO check if file's Swarm hash corresponds to the address
      filePath <- IO(dirPath.resolve(hash.toHex + ".wasm")).to[F]
      exists <- IO(filePath.toFile.exists()).to[F]
      _ <- if (exists) IO.unit.to[F]
      else
        for {
          tmpFile <- IO(Files.createTempFile("code_", "_wasm")).to[F]
          _ <- downloadToFile(hash, tmpFile)
          _ <- IO(Files.move(tmpFile, filePath)).to[F]
        } yield ()

    } yield dirPath

  /**
   * Downloads code from Swarm and manages paths to the code.
   * @param path a path to a code from the smart contract
   * @param workerPath a path to a worker's working directory
   * @return
   */
  override def prepareCode(path: CodePath, workerPath: Path): F[Path] =
    downloadAndWriteCodeToFile(workerPath, path.storageHash)

}
