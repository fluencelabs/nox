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

package fluence.node.workers.tendermint
import java.nio.file.Path

import cats.effect._
import fluence.node.docker.{DockerIO, DockerImage, DockerNetwork, DockerParams}
import fluence.node.workers.{DockerWorker, WorkerParams}
import fluence.node.workers.tendermint.config.ConfigTemplate

import scala.language.higherKinds

object DockerTendermint {
  // Internal ports
  val P2pPort: Short = 26656
  val RpcPort: Short = 26657
  val TmPrometheusPort: Short = 26660

  def execCmd[F[_]: Sync: ContextShift](
                                         tmImage: DockerImage,
                                         tendermintDir: Path,
                                         masterContainerId: Option[String],
                                         cmd: String,
                                         uid: String): F[String] =
    DockerIO.exec[F] {
      val params = DockerParams
        .build()
        .user(uid)

      masterContainerId match {
        case Some(cId) ⇒
          params
            .option("--volumes-from", cId)
            .option("-e", s"TMHOME=$tendermintDir")
            .image(tmImage)
            .runExec(cmd)

        case None ⇒
          params
            .volume(tendermintDir.toString, "/tendermint")
            .image(tmImage)
            .runExec(cmd)
      }
    }

  private def dockerCommand(
    params: WorkerParams,
    worker: DockerIO,
    network: DockerNetwork
  ): DockerParams.DaemonParams = {
    import params._

    val dockerParams = DockerParams
      .build()
      .option("-e", s"""TMHOME=$dataPath""")
      .option("--name", containerName(params))
      .option("--network", network.name)
      .port(currentWorker.p2pPort, P2pPort)
      .port(currentWorker.rpcPort, RpcPort)

    (masterNodeContainerId match {
      case Some(id) =>
        dockerParams.option("--volumes-from", s"$id:ro")
      case None =>
        dockerParams
    }).image(tmImage).daemonRun()
  }

  private[workers] def containerName(params: WorkerParams) =
    s"${params.appId}_tendermint_${params.currentWorker.index}"

  def make[F[_]: Sync: ContextShift: LiftIO](
    params: WorkerParams,
    worker: DockerIO,
    network: DockerNetwork
  ): Resource[F, DockerIO] =
    for {
      _ ← Resource.liftF(IO {
        ConfigTemplate.writeConfigs(params.configTemplate, params.app, params.dataPath, worker.containerId)
      }.to[F])
      container ← DockerIO.run[F](dockerCommand(params, worker, network))
    } yield container

}
