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

import cats.Applicative
import cats.effect._
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.node.docker._
import fluence.node.workers.WorkerParams
import fluence.node.workers.status.{HttpCheckNotPerformed, ServiceStatus}
import fluence.node.workers.tendermint.rpc.{TendermintRpc, TendermintStatus}

import scala.concurrent.duration.FiniteDuration
import scala.language.higherKinds

/**
 * Tendermint, running within Docker
 *
 * @param container Docker container
 * @param name Docker container's name, to connect to Tendermint via local network
 */
case class DockerTendermint(
  container: DockerIO,
  name: String
) {

  private def ifDockerOkRunHttpCheck[F[_]: Sync: ContextShift](
    rpc: TendermintRpc[F],
    dockerStatus: DockerStatus
  ): F[ServiceStatus[TendermintStatus]] =
    dockerStatus match {
      case d if d.isRunning ⇒ rpc.httpStatus.map(s ⇒ ServiceStatus(d, s))
      case d ⇒
        Applicative[F].pure(ServiceStatus(d, HttpCheckNotPerformed("Tendermint Docker container is not launched")))
    }

  /**
   * Service status for this docker + wrapped Tendermint Http service
   */
  def status[F[_]: Sync: ContextShift](rpc: TendermintRpc[F]): F[ServiceStatus[TendermintStatus]] =
    container.check[F].flatMap(ifDockerOkRunHttpCheck(rpc, _))

  /**
   * Launch service status check periodically. Resulting status is calculated from Docker container status and HTTP check.
   *
   * @param rpc Inner Tendermint RPC
   * @param period Perform check once in the specified period
   */
  def periodicalStatus[F[_]: Timer: Sync: ContextShift](
    rpc: TendermintRpc[F],
    period: FiniteDuration
  ): fs2.Stream[F, ServiceStatus[TendermintStatus]] =
    container.checkPeriodically[F](period).evalMap(ifDockerOkRunHttpCheck(rpc, _))

}

object DockerTendermint {
  // Internal ports
  val P2pPort: Short = 26656
  val RpcPort: Short = 26657
  val TmPrometheusPort: Short = 26660

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
  def execCmd[F[_]: Sync: ContextShift](
    tmDockerConfig: DockerConfig,
    tendermintDir: Path,
    masterContainerId: Option[String],
    cmd: String,
    uid: String
  ): F[String] =
    DockerIO.exec[F] {
      val params = DockerParams
        .build()
        .user(uid)

      masterContainerId match {
        case Some(cId) ⇒
          params
            .option("--volumes-from", cId)
            .option("-e", s"TMHOME=$tendermintDir")
            .prepared(tmDockerConfig)
            .runExec(cmd)

        case None ⇒
          params
            .volume(tendermintDir.toString, "/tendermint")
            .prepared(tmDockerConfig)
            .runExec(cmd)
      }
    }

  /**
   * Prepare a docker command for a particular Worker's Tendermint container
   */
  private def dockerCommand(
    params: WorkerParams,
    network: DockerNetwork,
    p2pPort: Short
  ): DockerParams.DaemonParams = {
    import params.{masterNodeContainerId, tendermintPath, tmImage}

    val dockerParams = DockerParams
      .build()
      .user("0") // TODO should only work when running from docker?
      .option("-e", s"""TMHOME=$tendermintPath""")
      .option("--name", containerName(params))
      .option("--network", network.name)
      .port(p2pPort, P2pPort)

    (masterNodeContainerId match {
      case Some(id) =>
        dockerParams
          .option("--volumes-from", id)
      case None =>
        dockerParams
    }).prepared(tmDockerConfig).daemonRun("node")
  }

  /**
   * Worker's Tendermint container's name
   */
  private def containerName(params: WorkerParams) =
    s"${params.appId}_tendermint_${params.currentWorker.index}"

  /**
   * Prepare and launch Tendermint container for the given Worker
   *
   * @param params Worker Params
   * @param workerName Docker name for the launched Worker (statemachine) container
   * @param network Worker's network
   * @param stopTimeout Seconds to wait for graceful stop of the Tendermint container before killing it
   * @return Running container
   */
  def make[F[_]: Sync: ContextShift: LiftIO](
    params: WorkerParams,
    p2pPort: Short,
    workerName: String,
    network: DockerNetwork,
    stopTimeout: Int
  ): Resource[F, DockerTendermint] =
    for {
      _ ← Resource.liftF(
        params.configTemplate.writeConfigs(params.app, params.tendermintPath, p2pPort, workerName)
      )
      container ← DockerIO.run[F](dockerCommand(params, network, p2pPort), stopTimeout)
    } yield DockerTendermint(container, containerName(params))

}
