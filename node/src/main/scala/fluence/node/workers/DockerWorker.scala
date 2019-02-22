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

import cats.{Applicative, Apply}
import cats.effect._
import cats.syntax.functor._
import cats.syntax.apply._
import cats.syntax.flatMap._
import com.softwaremill.sttp._
import fluence.node.docker._
import fluence.node.workers.control.ControlRpc
import fluence.node.workers.status._
import fluence.node.workers.tendermint.DockerTendermint
import fluence.node.workers.tendermint.rpc.TendermintRpc
import slogging.LazyLogging

import scala.language.higherKinds

/**
 * Single running worker's datatype
 *
 * @param p2pPort Tendermint p2p port
 * @param appId Worker's app ID
 * @param tendermint Tendermint RPC endpoints for the worker
 * @param control Control RPC endpoints for the worker
 * @param status Getter for actual Worker's status
 * @param stop stops the worker, should be launched only once
 * @param description human readable description of the Docker Worker
 * @tparam F the effect
 */
case class DockerWorker[F[_]] private (
  p2pPort: Short,
  appId: Long,
  tendermint: TendermintRpc[F],
  control: ControlRpc[F],
  status: F[WorkerStatus],
  stop: F[Unit],
  remove: F[Unit],
  description: String
) extends Worker[F]

object DockerWorker extends LazyLogging {
  val SmPrometheusPort: Short = 26661
  val ControlRpcPort: Short = 26662

  private def dockerCommand(params: WorkerParams, network: DockerNetwork): DockerParams.DaemonParams = {
    import params._

    val dockerParams = DockerParams
      .build()
      .option("-e", s"""CODE_DIR=$vmCodePath""")
      .option("--name", containerName(params))
      .option("--network", network.name)

    (masterNodeContainerId match {
      case Some(id) =>
        dockerParams.option("--volumes-from", s"$id:ro")
      case None =>
        dockerParams
    }).image(image).daemonRun()
  }

  private def dockerNetworkName(params: WorkerParams): String =
    s"fluence_${params.appId}_${params.currentWorker.index}"

  private def containerName(params: WorkerParams) =
    s"${params.appId}_worker_${params.currentWorker.index}"

  /**
   * Creates new docker network and connects node to that network
   *
   * @param params used for docker network name generation
   * @tparam F Effect
   * @return Resource of docker network and node connection.
   *         On release node will be disconnected, network will be removed.
   */
  private def makeNetwork[F[_]: ContextShift: Sync](params: WorkerParams): Resource[F, DockerNetwork] = {
    logger.debug(s"Creating docker network ${dockerNetworkName(params)} for $params")
    for {
      network <- DockerNetwork.make(dockerNetworkName(params))
      _ <- params.masterNodeContainerId.fold(Resource.pure(()))(DockerNetwork.join(_, network))
    } yield network
  }

  /**
   * Makes a single worker that runs once resource is in use
   *
   * @param params Worker's running params
   * @param p2pPort Tendermint p2p port
   * @param onStop A callback to launch when this worker is stopped
   * @param onRemove A callback to clean all the resources used by worker
   * @param stopTimeout Timeout in seconds to allow graceful stopping of running containers.
   *                    It might take up to 2*`stopTimeout` seconds to gracefully stop the worker, as 2 containers involved.
   * @param sttpBackend Sttp Backend to launch HTTP healthchecks and RPC endpoints
   * @return the [[Worker]] instance
   */
  def make[F[_]: ContextShift](
    params: WorkerParams,
    p2pPort: Short,
    onStop: F[Unit],
    onRemove: F[Unit],
    stopTimeout: Int
  )(
    implicit sttpBackend: SttpBackend[F, Nothing],
    F: Concurrent[F]
  ): Resource[F, Worker[F]] =
    for {
      network ← makeNetwork(params)

      worker ← DockerIO.run[F](dockerCommand(params, network), stopTimeout)

      tendermint ← DockerTendermint.make[F](params, p2pPort, containerName(params), network, stopTimeout)

      rpc ← TendermintRpc.make[F](tendermint.name, DockerTendermint.RpcPort)

      control = ControlRpc[F](containerName(params), ControlRpcPort)

      workerStatus = worker
        .check[F]
        .flatMap[ServiceStatus[Unit]] {
          case d if d.isRunning ⇒ control.status.map(s ⇒ ServiceStatus(d, s))
          case d ⇒
            Applicative[F].pure(ServiceStatus(d, HttpCheckNotPerformed("Worker's Docker container is not launched")))
        }

      status = Apply[F].map2(tendermint.status(rpc), workerStatus) { (ts, ws) ⇒
        WorkerStatus(
          isHealthy = ts.isOk(_.sync_info.latest_block_height > 1) && ws.isOk(),
          params.appId,
          ts,
          ws
        )
      }

    } yield
      new DockerWorker[F](p2pPort, params.appId, rpc, control, status, onStop, onStop *> onRemove, params.toString)

}
