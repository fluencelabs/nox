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

import java.nio.file.{Files, Path}

import cats.Monad
import cats.data.EitherT
import cats.effect.{IO, LiftIO}
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.effects.EffectError
import fluence.log.Log
import fluence.node.code.CodeCarrier
import fluence.worker.WorkerResource
import fluence.worker.eth.EthApp

import scala.language.higherKinds

case class WorkerFiles[F[_]: Monad: LiftIO](
  rootPath: Path,
  codeCarrier: CodeCarrier[F]
) {

  /**
   * All app worker's data is stored here. Currently the folder is never purged
   */
  private def resolveAppPath(app: EthApp): F[Path] =
    IO(rootPath.resolve("app-" + app.id + "-" + app.cluster.currentWorker.index)).to[F]

  /**
   * Create directory to hold Tendermint config & data for a specific app (worker)
   *
   * @param appPath Path containing all configs & data for a specific app
   * @return Path to Tendermint home ($TMHOME) directory
   */
  private def makeTendermintPath(appPath: Path): F[Path] =
    for {
      tendermintPath ← IO(appPath.resolve("tendermint")).to[F]
      _ ← IO(Files.createDirectories(tendermintPath)).to[F]
    } yield tendermintPath

  /**
   * Create directory to hold app code downloaded from Swarm
   *
   * @param appPath Path containing all configs & data for a specific app
   * @return Path to `vmcode` directory
   */
  private def makeVmCodePath(appPath: Path): F[Path] =
    for {
      vmCodePath ← IO(appPath.resolve("vmcode")).to[F]
      _ ← IO(Files.createDirectories(vmCodePath)).to[F]
    } yield vmCodePath

  def workerResource(app: EthApp): WorkerResource[F, WorkerFiles.Paths[F]] =
    new WorkerResource[F, WorkerFiles.Paths[F]] {
      override def prepare()(implicit log: Log[F]): F[WorkerFiles.Paths[F]] =
        for {
          appPath ← resolveAppPath(app)
          tendermint ← makeTendermintPath(appPath)
        } yield WorkerFiles.Paths(
          makeVmCodePath(appPath).flatMap(vmCodePath ⇒ codeCarrier.carryCode(app.code, vmCodePath)),
          tendermint
        )

      // TODO clean folders
      override def destroy()(implicit log: Log[F]): EitherT[F, EffectError, Unit] =
        EitherT.pure(())
    }

}

object WorkerFiles {
  case class Paths[F[_]](
    code: F[Path],
    tendermint: Path
  )
}
