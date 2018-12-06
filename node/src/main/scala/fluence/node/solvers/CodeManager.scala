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

package fluence.node.solvers
import java.nio.file.{Files, Path, Paths}

import cats.effect.Sync
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.ethclient.helpers.Web3jConverters.{binaryToHex, bytes32ToString}
import fluence.swarm.SwarmClient
import org.web3j.abi.datatypes.generated.Bytes32

import scala.language.higherKinds

case class CodePath(storageHash: Bytes32) {
  lazy val asString: String = bytes32ToString(storageHash)
  lazy val asHex: String = binaryToHex(storageHash.getValue)
}

sealed trait CodeManager[F[_]] {

  /**
   * Downloads code from Swarm and manages paths to the code.
   * @param path a path to a code from the deployer smart contract
   * @param storagePath a path to a solver working directory
   * @return
   */
  def prepareCode(path: CodePath, storagePath: Path): F[String]
}

/**
 * Manager only for test purposes, uses precompiled code from Fluence repository.
 *
 */
class TestCodeManager[F[_]](implicit F: Sync[F]) extends CodeManager[F] {

  /**
   * Downloads code from Swarm and manages paths to the code.
   * @param path a path to a code from the deployer smart contract
   * @param solverPath a path to a solver working directory
   * @return
   */
  override def prepareCode(
    path: CodePath,
    solverPath: Path
  ): F[String] = F.pure("/master/vmcode/vmcode-" + path.asString) // preloaded code in master's docker container
}

/**
 * Uses the Swarm network to download a code.
 *
 */
class SwarmCodeManager[F[_]](swarmClient: SwarmClient[F])(implicit F: Sync[F]) extends CodeManager[F] {

  /**
   * Downloads file from the Swarm and store it on a disk.
   * @param swarmPath a code address and a Swarm URL address
   * @param filePath a path to code to store
   */
  private def downloadFromSwarmToFile(swarmPath: String, filePath: Path): F[Unit] = {
    //TODO change this to return stream from `download` method
    swarmClient.download(swarmPath).value.flatMap {
      case Left(err) => F.raiseError(err)
      case Right(codeBytes) => F.delay(Files.write(filePath, codeBytes))
    }
  }

  /**
   * Checks if there is no code already then download a file from the Swarm and store it to a disk.
   * @param solverPath a path to solver's directory
   * @param swarmPath a code address and a Swarm URL address
   * @return a path to a code
   */
  private def downloadAndWriteCodeToFile(
    solverPath: Path,
    swarmPath: String
  ): F[String] =
    for {
      dirPath <- F.delay(solverPath.resolve("vmcode"))
      _ <- if (dirPath.toFile.exists()) F.unit else F.delay(Files.createDirectory(dirPath))
      //TODO check if file's Swarm hash corresponds to the address
      filePath <- F.delay(dirPath.resolve(swarmPath + ".wasm"))
      exists <- F.delay(filePath.toFile.exists())
      _ <- if (exists) F.unit
      else
        F.delay(Files.createFile(filePath))
          .flatMap(_ => downloadFromSwarmToFile(swarmPath, filePath))
    } yield dirPath.toAbsolutePath.toString

  /**
   * Downloads code from Swarm and manages paths to the code.
   * @param path a path to a code from the deployer smart contract
   * @param solverPath a path to a solver working directory
   * @return
   */
  override def prepareCode(path: CodePath, solverPath: Path): F[String] = {
    downloadAndWriteCodeToFile(solverPath, path.asHex)
  }
}
