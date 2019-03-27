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

import java.nio.file.{Files, Path, Paths}

import cats.effect.concurrent.MVar
import cats.effect.{Concurrent, Sync, Timer}
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.{Applicative, Monad}
import com.softwaremill.sttp.{asByteArray, sttp, SttpBackend, Uri}
import fluence.effects.swarm.helpers.ResponseOps._
import fluence.effects.swarm.{SwarmClient, SwarmError}
import fluence.effects.syntax.backoff._
import fluence.effects.{Backoff, EffectError}
import fluence.node.config.SwarmConfig
import scodec.bits.ByteVector

import scala.language.higherKinds

case class CodePath(storageHash: ByteVector) {
  lazy val asHex: String = storageHash.toHex
}

sealed trait CodeManager[F[_]] {

  /**
   * Downloads code from Swarm and manages paths to the code.
   *
   * @param path a path to a code from the smart contract
   * @param storagePath a path to a worker's working directory
   * @return
   */
  def prepareCode(path: CodePath, storagePath: Path): F[Path]
}

/**
 * Manager only for test purposes, uses precompiled code from Fluence repository.
 *
 */
class TestCodeManager[F[_]](implicit F: Sync[F]) extends CodeManager[F] {

  /**
   * Downloads code from Swarm and manages paths to the code.
   *
   * @param path a path to a code from the smart contract
   * @param workerPath a path to a worker's working directory
   * @return
   */
  override def prepareCode(
    path: CodePath,
    workerPath: Path
  ): F[Path] =
    F.fromEither(path.storageHash.decodeUtf8.map(_.trim))
      .flatMap(p => F.pure(Paths.get("/master/vmcode/vmcode-" + p))) // preloaded code in master's docker container
}

class PolyglotCodeManager[F[_]: Timer: Monad: Sync: Concurrent](swarm: SwarmCodeManager[F], ipfs: IpfsCodeManager[F])
    extends CodeManager[F] {
  override def prepareCode(
    path: CodePath,
    storagePath: Path
  ): F[Path] =
    for {
      mvar <- MVar.empty[F, Path]
      swarmTmp <- Sync[F].delay(Files.createTempDirectory("swarm"))
      fromSwarm <- Concurrent[F].start(swarm.prepareCode(path, swarmTmp).flatTap(mvar.tryPut))
      ipfsTmp <- Sync[F].delay(Files.createTempDirectory("ipfs"))
      fromIpfs <- Concurrent[F].start(ipfs.prepareCode(path, ipfsTmp).flatTap(mvar.tryPut))
      tmp <- mvar.read
      _ <- fromSwarm.cancel
      _ <- fromIpfs.cancel
      result <- Sync[F].delay(Files.move(tmp, storagePath))
    } yield result
}

case class IpfsError(message: String, causedBy: Option[Throwable] = None) extends EffectError {
  override def getMessage: String = message

  override def getCause: Throwable = causedBy getOrElse super.getCause
}

class IpfsCodeManager[F[_]: Timer: Monad](ipfsUri: Uri)(
  implicit F: Sync[F],
  backoff: Backoff[SwarmError],
  sttpBackend: SttpBackend[F, Nothing]
) {

  def prepareCode(path: CodePath, storagePath: Path): F[Path] = {
    for {
      dirPath <- F.delay(storagePath.resolve("vmcode"))
      _ <- if (dirPath.toFile.exists()) F.unit else F.delay(Files.createDirectory(dirPath))

      //TODO check if file's Swarm hash corresponds to the address
      filePath <- F.delay(dirPath.resolve(path + ".wasm"))
      exists <- F.delay(filePath.toFile.exists())
      _ <- if (exists) F.unit
      else {
        for {
          tmpFile <- F.delay(Files.createTempFile("code_", "_wasm"))
          _ <- download(path, tmpFile)
          _ <- F.delay(Files.move(tmpFile, filePath))
        } yield ()
      }
    } yield dirPath
  }

  def download(code: CodePath, target: Path): F[Unit] = {
    val address = (ByteVector(0x12, 0x20) ++ code.storageHash).toBase58
    val uri = ipfsUri.path("/api/v0/get").param("arg", address)
    sttp
      .response(asByteArray)
      .get(uri)
      .send()
      .toEitherT { er =>
        val errorMessage = s"Error on downloading from $uri. $er"
        SwarmError(errorMessage)
      }
      .backoff
      .flatMap { codeBytes =>
        F.delay(Files.write(target, codeBytes))
      }
  }
}

/**
 * Uses the Swarm network to download a code.
 *
 */
class SwarmCodeManager[F[_]: Timer](swarmClient: SwarmClient[F])(implicit F: Sync[F], backoff: Backoff[SwarmError])
    extends CodeManager[F] {

  /**
   * Downloads file from the Swarm and store it on a disk.
   *
   * @param swarmPath a code address and a Swarm URL address
   * @param filePath a path to code to store
   */
  private def downloadFromSwarmToFile(swarmPath: String, filePath: Path): F[Unit] =
    //TODO change this to return stream from `download` method
    swarmClient.download(swarmPath).backoff.flatMap { codeBytes =>
      F.delay(Files.write(filePath, codeBytes))
    }

  /**
   * Checks if there is no code already then download a file from the Swarm and store it to a disk.
   *
   * @param workerPath a path to worker's directory
   * @param swarmPath a code address and a Swarm URL address
   * @return a path to a code
   */
  private def downloadAndWriteCodeToFile(
    workerPath: Path,
    swarmPath: String
  ): F[Path] =
    // TODO handle fail system errors properly?
    for {
      // TODO why to hardcode the directory and its creation?
      dirPath <- F.delay(workerPath.resolve("vmcode"))
      _ <- if (dirPath.toFile.exists()) F.unit else F.delay(Files.createDirectory(dirPath))

      //TODO check if file's Swarm hash corresponds to the address
      filePath <- F.delay(dirPath.resolve(swarmPath + ".wasm"))
      exists <- F.delay(filePath.toFile.exists())
      _ <- if (exists) F.unit
      else {
        for {
          tmpFile <- F.delay(Files.createTempFile("code_", "_wasm"))
          _ <- downloadFromSwarmToFile(swarmPath, tmpFile)
          _ <- F.delay(Files.move(tmpFile, filePath))
        } yield ()
      }
    } yield dirPath

  /**
   * Downloads code from Swarm and manages paths to the code.
   *
   * @param path a path to a code from the smart contract
   * @param workerPath a path to a worker's working directory
   * @return
   */
  override def prepareCode(path: CodePath, workerPath: Path): F[Path] = {
    downloadAndWriteCodeToFile(workerPath, path.asHex)
  }
}

object CodeManager {

  def apply[F[_]: Sync: Timer: Concurrent](
    config: SwarmConfig
  )(implicit sttpBackend: SttpBackend[F, Nothing], backoff: Backoff[SwarmError] = Backoff.default): F[CodeManager[F]] =
    if (config.enabled) {
      for {
        swarm <- SwarmClient(config.address).map(new SwarmCodeManager[F](_))
        ipfs = new IpfsCodeManager[F](Uri(config.address.replace("8500", "5001")))
      } yield new PolyglotCodeManager[F](swarm, ipfs)
    } else {
      Applicative[F].pure(new TestCodeManager[F]())
    }
}
