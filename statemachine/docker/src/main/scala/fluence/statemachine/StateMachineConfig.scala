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

package fluence.statemachine

import java.io.File

import cats.data.{EitherT, NonEmptyList}
import cats.effect.{IO, LiftIO, Sync}
import cats.instances.list._
import cats.syntax.applicativeError._
import cats.syntax.flatMap._
import cats.syntax.list._
import cats.{Monad, Traverse}
import com.typesafe.config.{Config, ConfigFactory}
import fluence.statemachine.error.{StateMachineError, VmModuleLocationError}
import net.ceedubs.ficus.readers.ValueReader

import scala.language.higherKinds

/**
 * State machine settings.
 *
 * @param sessionExpirationPeriod Period after which the session becomes expired,
 *                                measured as difference between the current `txCounter` value and
 *                                its value at the last activity in the session.
 * @param moduleFiles Sequence of files with WASM module code
 * @param logLevel Level of logging ( OFF / ERROR / WARN / INFO / DEBUG / TRACE )
 * @param abciPort Port to listen for ABCI events
 * @param http Configuration for ControlRPC server
 * @param blockUploadingEnabled Whether to retrieve block receipts and use them in app hash or not
 */
case class StateMachineConfig(
  sessionExpirationPeriod: Long,
  moduleFiles: List[String],
  logLevel: String,
  abciPort: Short,
  http: HttpConfig,
  blockUploadingEnabled: Boolean
) {

  /**
   * Extracts module filenames that have wast or wasm extensions from the module-files section of a given config.
   *
   * @return either a sequence of filenames found in directories and among files provided in config
   *         or error denoting a specific problem with locating one of directories and files from config
   */
  def collectModuleFiles[F[_]: LiftIO: Monad]: EitherT[F, StateMachineError, NonEmptyList[String]] =
    EitherT(
      Traverse[List]
        .flatTraverse(moduleFiles)(
          StateMachineConfig.listWasmFiles
        )
        // convert flattened list of file paths to nel (IO[List[String]] => IO[Option[NonEmptyList[String]]])
        .map(_.toNel)
        .attempt
        .to[F]
    ).leftMap { e =>
      VmModuleLocationError("Error during locating VM module files and directories", Some(e))
    }.subflatMap(
      // EitherT[F, E, Option[NonEmptyList[String]]]] => EitherT[F, E, NonEmptyList[String]]]
      _.toRight[StateMachineError](
        VmModuleLocationError("Provided directories don't contain any wasm or wast files")
      )
    )

}

object StateMachineConfig {

  /**
   * Loads State machine config using Typesafe Config with Ficus.
   */
  def load[F[_]: Sync](conf: ⇒ Config = ConfigFactory.load()): F[StateMachineConfig] = {
    import net.ceedubs.ficus.Ficus._
    import net.ceedubs.ficus.readers.ArbitraryTypeReader._
    import net.ceedubs.ficus.readers.namemappers.implicits.hyphenCase

    implicit val shortValueReader: ValueReader[Short] =
      ValueReader[Long].map(_.toShort)

    Sync[F]
      .delay(
        conf.as[StateMachineConfig]
      )
      .attempt
      .flatMap {
        case Left(err) ⇒
          Sync[F].raiseError(new RuntimeException("Unable to parse StateMachineConfig: " + err))

        case Right(c) ⇒
          Sync[F].pure(c)
      }
  }

  /**
   * Collects and returns all files in given folder
   *
   * @param path a path to a folder where files should be listed
   * @return a list of files in given directory or provided file if the path to a file has has been given
   */
  def listFiles(path: String): IO[List[File]] = IO {
    val pathName = new File(path)
    pathName match {
      case file if pathName.isFile     => file :: Nil
      case dir if pathName.isDirectory => Option(dir.listFiles).fold(List.empty[File])(_.toList)
    }
  }

  /**
   * List files in the given folder, keep only .wasm and .wast ones
   *
   * @param path Folder to walk through
   * @return List of found files, possibly empty
   */
  def listWasmFiles(path: String): IO[List[String]] =
    listFiles(path)
      .map(
        // converts File objects to their path
        _.map(_.getPath)
        // filters out non-Wasm files
          .filter(filePath => filePath.endsWith(".wasm") || filePath.endsWith(".wast"))
      )
}
