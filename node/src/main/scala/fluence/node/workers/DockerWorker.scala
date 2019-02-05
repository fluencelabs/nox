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

import cats.Applicative
import cats.effect._
import cats.effect.concurrent.Ref
import cats.syntax.functor._
import com.softwaremill.sttp._
import fluence.node.MakeResource
import fluence.node.docker.{DockerIO, DockerNetwork, DockerParams}
import fluence.node.eth.state.WorkerPeer
import fluence.node.workers.control.ControlRpc
import fluence.node.workers.health._
import fluence.node.workers.tendermint.rpc.TendermintRpc
import slogging.LazyLogging

import scala.language.higherKinds
import scala.concurrent.duration.MILLISECONDS

/**
 * Single running worker's datatype
 *
 * @param params this worker's description
 * @param tendermint Tendermint RPC endpoints for the worker
 * @param control Control RPC endpoints for the worker
 * @param healthReportRef a reference to the last healthcheck, updated every time a new healthcheck is being made
 * @param stop stops the worker, should be launched only once
 * @tparam F the effect
 */
case class DockerWorker[F[_]] private (
  tendermint: TendermintRpc[F],
  control: ControlRpc[F],
  private val healthReportRef: Ref[F, WorkerHealth],
  stop: F[Unit],
  description: String
) extends Worker[F] {

  // Getter for the last healthcheck
  val healthReport: F[WorkerHealth] = healthReportRef.get
}

object DockerWorker extends LazyLogging {
  // Internal ports
  val P2P_PORT: Short = 26656
  val RPC_PORT: Short = 26657
  val TM_PROMETHEUS_PORT: Short = 26660
  val SM_PROMETHEUS_PORT: Short = 26661

  private def dockerCommand(params: WorkerParams, network: DockerNetwork): DockerParams.DaemonParams = {
    import params._

    val dockerParams = DockerParams
      .build()
      .option("-e", s"""CODE_DIR=$vmCodePath""")
      .option("-e", s"""WORKER_DIR=$workerPath""")
      .option("--name", containerName(params))
      .option("--network", network.name)
      .port(currentWorker.p2pPort, P2P_PORT)
      .port(currentWorker.rpcPort, RPC_PORT)

    (masterNodeContainerId match {
      case Some(id) => dockerParams.option("--volumes-from", s"$id:ro")
      case None => dockerParams
    }).image(image).daemonRun()
  }

  private def dockerNetworkName(params: WorkerParams): String =
    s"fluence_${params.appId}_${params.currentWorker.index}"

  private def containerName(params: WorkerParams) =
    s"${params.appId}_worker_${params.currentWorker.index}"

  /**
   * For each successful container's status check, runs HTTP request for Tendermint status, and provides a WorkerHealth report
   * TODO: we have [[HealthCheckConfig.slide]] to act only when a defined fraction of last healthchecks failed. However, we don't use it
   * as we currently have no behavior definition for worker's failure: we don't try to restart it, nor we stop it.
   *
   * @param container Running Worker container
   * @param params Worker params to include into WorkerHealth object
   * @param healthCheckConfig See [[HealthCheckConfig]]
   * @param rpc Tendermint RPC
   * @tparam F Effect
   * @return Stream of periodical health reports
   */
  private def healthCheckStream[F[_]: Timer: Sync: ContextShift](
    container: DockerIO,
    params: WorkerParams,
    healthCheckConfig: HealthCheckConfig,
    rpc: TendermintRpc[F]
  ): fs2.Stream[F, WorkerHealth] =
    container
      .checkPeriodically[F](healthCheckConfig.period)
      .evalMap(
        st ⇒
          // Calculate the uptime
          Timer[F].clock
            .realTime(MILLISECONDS)
            .map(now ⇒ (now - st.startedAt) → st.isRunning)
      )
      .evalMap[F, WorkerHealth] {
        case (uptime, true) ⇒
          rpc.status.value.map {
            case Left(err) ⇒
              logger.error("Worker HTTP check failed: " + err.getLocalizedMessage, err)
              WorkerHttpCheckFailed(StoppedWorkerInfo(params.currentWorker), err)

            case Right(tendermintInfo) ⇒
              val info = RunningWorkerInfo(params, tendermintInfo)
              WorkerRunning(uptime, info)
          }.map { health ⇒
            logger.debug(s"HTTP health is: $health")
            health
          }

        case (_, false) ⇒
          logger.error(s"Healthcheck is failing for worker: $params")
          Applicative[F].pure(WorkerContainerNotRunning(StoppedWorkerInfo(params.currentWorker)))
      }

  private def makeNetwork[F[_]: ContextShift: Sync](params: WorkerParams): Resource[F, DockerNetwork] = {
    logger.debug(s"Creating docker network ${dockerNetworkName(params)} for $params")
    for {
      network <- DockerNetwork.make(dockerNetworkName(params))
//      _ <- params.masterNodeContainerId.fold(Resource.pure(()))(DockerNetwork.join(_, network))
      _ <- DockerNetwork.join(params.masterNodeContainerId.get, network)
    } yield network
  }

  /**
   * Makes a single worker that runs once resource is in use
   *
   * @param params Worker's running params
   * @param healthCheckConfig see [[HealthCheckConfig]]
   * @param onStop A callback to launch when this worker is stopped
   * @param sttpBackend Sttp Backend to launch HTTP healthchecks and RPC endpoints
   * @return the [[Worker]] instance
   */
  def make[F[_]: ContextShift: Timer](
    params: WorkerParams,
    healthCheckConfig: HealthCheckConfig,
    onStop: F[Unit]
  )(
    implicit sttpBackend: SttpBackend[F, Nothing],
    F: Concurrent[F]
  ): Resource[F, Worker[F]] =
    for {
      healthReportRef ← MakeResource.refOf[F, WorkerHealth](
        WorkerNotYetLaunched(StoppedWorkerInfo(params.currentWorker))
      )

      network ← makeNetwork(params)
      container ← DockerIO.run[F](dockerCommand(params, network))
      rpc ← TendermintRpc.make[F](containerName(params), RPC_PORT)

      healthChecks = healthCheckStream(container, params, healthCheckConfig, rpc)

      // Runs health checker, wrapped with resource:
      // health check will be stopped when the resource is released.
      _ ← MakeResource.concurrentStream[F](healthChecks.evalTap(healthReportRef.set))

      control = ControlRpc[F]()

    } yield new DockerWorker[F](rpc, control, healthReportRef, onStop, params.toString)

}
