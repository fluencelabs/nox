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

import cats.data.EitherT
import cats.effect._
import fluence.bp.api.{BlockProducer, BlockStream, DialPeers}
import fluence.bp.tendermint.{Tendermint, TendermintBlockProducer}
import fluence.effects.EffectError
import fluence.effects.docker._
import fluence.effects.docker.params.DockerParams
import fluence.effects.sttp.SttpEffect
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.rpc.websocket.WebsocketConfig
import fluence.log.Log
import fluence.node.config.DockerConfig
import fluence.node.workers.WorkerDocker
import fluence.node.workers.tendermint.config.ConfigTemplate
import fluence.worker.eth.EthApp
import shapeless._

import scala.language.higherKinds

object DockerTendermint {
  // Internal ports
  val P2pPort: Short = 26656
  val RpcPort: Short = 26657

  /**
   * Execute tendermint-specific command inside a temporary container, return the results
   *
   * @param tmDockerConfig Tendermint image to use, with cpu and memory limits
   * @param tendermintDir Tendermint's home directory in current filesystem
   * @param masterContainerId Master process' Docker ID, used to pass volumes from
   * @param cmd Command to run
   * @param uid User ID to run Tendermint with; must have access to `tendermintDir`
   * @tparam F Effect
   * @return String output of the command execution
   */
  def execCmd[F[_]: DockerIO: Log](
    tmDockerConfig: DockerConfig,
    tendermintDir: Path,
    masterContainerId: Option[String],
    cmd: String,
    uid: String
  ): EitherT[F, DockerError, String] =
    DockerIO[F].exec {
      val params = DockerParams
        .build()
        .user(uid)
        .limits(tmDockerConfig.limits)

      masterContainerId match {
        case Some(cId) ⇒
          params
            .option("--volumes-from", cId)
            .option("-e", s"TMHOME=$tendermintDir")
            .prepared(tmDockerConfig.image)
            .runExec(cmd)

        case None ⇒
          params
            .volume(tendermintDir.toString, "/tendermint")
            .prepared(tmDockerConfig.image)
            .runExec(cmd)
      }
    }

  /**
   * Prepare a docker command for a particular Worker's Tendermint container
   */
  private def dockerCommand(
    masterNodeContainerId: Option[String],
    component: WorkerDocker.Component,
    tendermintPath: Path,
    network: DockerNetwork,
    p2pPort: Short
  ): DockerParams.DaemonParams = {
    val dockerParams = DockerParams
      .build()
      .user("0") // TODO should only work when running from docker?
      .option("-e", s"""TMHOME=$tendermintPath""")
      .option("--name", component.name)
      .option("--network", network.name)
      .port(p2pPort, P2pPort)
      .limits(component.docker.limits)

    (masterNodeContainerId match {
      case Some(id) =>
        dockerParams
          .option("--volumes-from", id)
      case None =>
        dockerParams
    }).prepared(component.docker.image).daemonRun("node")
  }

  /**
   * Prepare and launch Tendermint container for the given Worker
   *
   * @return Running container
   */
  def make[F[_]: DockerIO: LiftIO: ConcurrentEffect: SttpEffect: Timer: ContextShift: Log](
    app: EthApp,
    tendermintPath: Path,
    configTemplate: ConfigTemplate,
    workerDocker: WorkerDocker,
    p2pPort: Short,
    websocketConfig: WebsocketConfig
  ): Resource[F, BlockProducer.Aux[F, DockerContainer :: BlockStream[F, Block] :: DialPeers[F] :: HNil]] =
    for {
      // TODO make it WorkerResource
      _ ← Resource.liftF(
        configTemplate.writeConfigs(app, tendermintPath, p2pPort, workerDocker.machine.name)
      )
      container ← DockerIO[F].run(
        dockerCommand(
          workerDocker.masterContainerId,
          workerDocker.producer,
          tendermintPath,
          workerDocker.network,
          p2pPort
        ),
        workerDocker.stopTimeout
      )

      tm ← Tendermint.make[F](workerDocker.producer.name, DockerTendermint.RpcPort, tendermintPath, websocketConfig)

    } yield TendermintBlockProducer(
      tm,
      DockerIO[F].checkContainer(container).leftMap(identity[EffectError])
    ).extend(container)

}
