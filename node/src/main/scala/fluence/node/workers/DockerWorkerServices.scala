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

import java.nio.file.Path

import cats.data.EitherT
import cats.effect._
import cats.syntax.functor._
import cats.{Apply, Monad, Parallel}
import com.softwaremill.sttp._
import fluence.effects.docker._
import fluence.effects.docker.params.DockerParams
import fluence.log.Log
import fluence.effects.tendermint.rpc.TendermintRpc
import fluence.log.LogLevel.LogLevel
import fluence.node.workers.control.ControlRpc
import fluence.node.workers.status._
import fluence.node.workers.subscription.ResponseSubscriber
import fluence.node.workers.tendermint.DockerTendermint

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
  blockManifests: WorkerBlockManifests[F],
  responseSubscriber: ResponseSubscriber[F],
  statusCall: FiniteDuration ⇒ F[WorkerStatus]
) extends WorkerServices[F] {
  override def status(timeout: FiniteDuration): F[WorkerStatus] = statusCall(timeout)
}

object DockerWorkerServices {
  val ControlRpcPort: Short = 26662

  private def dockerCommand(params: WorkerParams,
                            network: DockerNetwork,
                            logLevel: LogLevel): DockerParams.DaemonParams = {
    import params._

    // Set worker's Xmx to mem * 0.75, so there's a gap between JVM heap and cgroup memory limit
    val internalMem = dockerConfig.limits.memoryMb.map(mem => Math.floor(mem * 0.75).toInt)

    DockerParams
      .build()
      .option("-e", s"""CODE_DIR=$vmCodePath""")
      .option("-e", s"LOG_LEVEL=$logLevel")
      .option("-e", s"TM_RPC_PORT=${DockerTendermint.RpcPort}")
      .option("-e", s"TM_RPC_HOST=${DockerTendermint.containerName(params)}")
      .option("-e", internalMem.map(mem => s"WORKER_MEMORY_LIMIT=$mem"))
      .option("--name", containerName(params))
      .option("--network", network.name)
      .option("--volumes-from", masterNodeContainerId.map(id => s"$id:ro"))
      .limits(dockerConfig.limits)
      .prepared(dockerConfig.image)
      .daemonRun()
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
  private def makeNetwork[F[_]: DockerIO: Monad: Log](params: WorkerParams): Resource[F, DockerNetwork] =
    for {
      _ ← Log.resource[F].debug(s"Creating docker network ${dockerNetworkName(params)} for $params")
      network <- DockerNetwork.make(dockerNetworkName(params))
      _ <- params.masterNodeContainerId
        .map(DockerContainer(_, None))
        .fold(Resource.pure(()))(DockerNetwork.join(_, network))
    } yield network

  /**
   * Makes a single worker that runs once resource is in use
   *
   * @param params Worker's running params
   * @param p2pPort Tendermint p2p port
   * @param stopTimeout Timeout in seconds to allow graceful stopping of running containers.
   *                    It might take up to 2*`stopTimeout` seconds to gracefully stop the worker, as 2 containers involved.
   * @param logLevel Logging level passed to the worker
   * @param storageRootPath Storage root, to be used with [[KVReceiptStorage.make]]
   * @param sttpBackend Sttp Backend to launch HTTP healthchecks and RPC endpoints
   * @return the [[WorkerServices]] instance
   */
  def make[F[_]: DockerIO: Timer: ConcurrentEffect: Log: ContextShift, G[_]](
    params: WorkerParams,
    p2pPort: Short,
    stopTimeout: Int,
    logLevel: LogLevel,
    storageRootPath: Path
  )(
    implicit sttpBackend: SttpBackend[EitherT[F, Throwable, ?], Nothing],
    F: Concurrent[F],
    P: Parallel[F, G]
  ): Resource[F, WorkerServices[F]] =
    for {
      network ← makeNetwork(params)

      worker ← DockerIO[F].run(dockerCommand(params, network, logLevel), stopTimeout)

      tendermint ← DockerTendermint.make[F](params, p2pPort, containerName(params), network, stopTimeout)

      rpc ← TendermintRpc.make[F](tendermint.name, DockerTendermint.RpcPort)

      blockManifests ← WorkerBlockManifests.make[F](params.appId, storageRootPath)

      responseSubscriber <- ResponseSubscriber.make(rpc, params.appId)

      control = ControlRpc[F](containerName(params), ControlRpcPort)

      workerStatus = (timeout: FiniteDuration) ⇒
        DockerIO[F]
          .checkContainer(worker)
          .semiflatMap[ServiceStatus[Unit]] { d ⇒
            HttpStatus
              .timed(control.status, timeout)
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

    } yield new DockerWorkerServices[F](p2pPort, params.appId, rpc, control, blockManifests, responseSubscriber, status)

}
