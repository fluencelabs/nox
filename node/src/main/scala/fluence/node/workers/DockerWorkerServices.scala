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

import cats.{Apply, Monad}
import cats.effect._
import cats.syntax.functor._
import cats.syntax.apply._
import cats.syntax.flatMap._
import com.softwaremill.sttp._
import fluence.effects.docker._
import fluence.effects.docker.params.DockerParams
import fluence.node.workers.control.ControlRpc
import fluence.node.workers.status._
import fluence.node.workers.tendermint.DockerTendermint
import fluence.effects.tendermint.rpc.TendermintRpc
import slogging.LazyLogging

import scala.concurrent.duration.FiniteDuration
import scala.language.higherKinds

/**
 * Single running worker's datatype
 *
 * @param p2pPort Tendermint p2p port
 * @param appId Worker's app ID
 * @param tendermint Tendermint RPC endpoints for the worker
 * @param control Control RPC endpoints for the worker
 * @param statusCall Getter for actual Worker's status
 * @tparam F the effect
 */
case class DockerWorkerServices[F[_]] private (
  p2pPort: Short,
  appId: Long,
  tendermint: TendermintRpc[F],
  control: ControlRpc[F],
  statusCall: FiniteDuration ⇒ F[WorkerStatus]
) extends WorkerServices[F] {
  override def status(timeout: FiniteDuration): F[WorkerStatus] = statusCall(timeout)
}

object DockerWorkerServices extends LazyLogging {
  val ControlRpcPort: Short = 26662

  private def dockerCommand(params: WorkerParams, network: DockerNetwork): DockerParams.DaemonParams = {
    import params._

    val dockerParams = DockerParams
      .build()
      .option("-e", s"""CODE_DIR=$vmCodePath""")
      .option("-e", s"TM_RPC_PORT=${DockerTendermint.RpcPort}")
      .option("-e", s"TM_RPC_HOST=${DockerTendermint.containerName(params)}")
      .option("--name", containerName(params))
      .option("--network", network.name)
      .limits(dockerConfig.limits)

    (masterNodeContainerId match {
      case Some(id) =>
        dockerParams.option("--volumes-from", s"$id:ro")
      case None =>
        dockerParams
    }).prepared(dockerConfig.image).daemonRun()
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
  private def makeNetwork[F[_]: DockerIO: Monad](params: WorkerParams): Resource[F, DockerNetwork] = {
    logger.debug(s"Creating docker network ${dockerNetworkName(params)} for $params")
    for {
      network <- DockerNetwork.make(dockerNetworkName(params))
      _ <- params.masterNodeContainerId.map(DockerContainer).fold(Resource.pure(()))(DockerNetwork.join(_, network))
    } yield network
  }

  /**
   * Makes a single worker that runs once resource is in use
   *
   * @param params Worker's running params
   * @param p2pPort Tendermint p2p port
   * @param stopTimeout Timeout in seconds to allow graceful stopping of running containers.
   *                    It might take up to 2*`stopTimeout` seconds to gracefully stop the worker, as 2 containers involved.
   * @param sttpBackend Sttp Backend to launch HTTP healthchecks and RPC endpoints
   * @return the [[WorkerServices]] instance
   */
  def make[F[_]: DockerIO: Timer](
    params: WorkerParams,
    p2pPort: Short,
    stopTimeout: Int
  )(
    implicit sttpBackend: SttpBackend[F, Nothing],
    F: Concurrent[F]
  ): Resource[F, WorkerServices[F]] =
    for {
      network ← makeNetwork(params)

      worker ← DockerIO[F].run(dockerCommand(params, network), stopTimeout)

      tendermint ← DockerTendermint.make[F](params, p2pPort, containerName(params), network, stopTimeout)

      rpc ← TendermintRpc.make[F](tendermint.name, DockerTendermint.RpcPort)

      control = ControlRpc[F](containerName(params), ControlRpcPort)

      workerStatus = (timeout: FiniteDuration) ⇒
        DockerIO[F]
          .checkContainer(worker)
          .semiflatMap[ServiceStatus[Unit]] { d ⇒
            HttpStatus
              .unhalt(control.status, timeout)
              .map(s ⇒ ServiceStatus(Right(d), s))
          }
          .valueOr(err ⇒ ServiceStatus(Left(err), HttpCheckNotPerformed("Worker's Docker container is not launched")))

      status = (timeout: FiniteDuration) ⇒
        Apply[F].map2(tendermint.status(rpc, timeout), workerStatus(timeout)) { (ts, ws) ⇒
          WorkerStatus(
            isHealthy = ts.isOk(_.sync_info.latest_block_height > 1) && ws.isOk(),
            params.appId,
            ts,
            ws
          )
      }

    } yield new DockerWorkerServices[F](p2pPort, params.appId, rpc, control, status)

}
