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
import fluence.node.docker.DockerIO
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
case class Worker[F[_]] private (
  params: WorkerParams,
  tendermint: TendermintRpc[F],
  control: ControlRpc[F],
  private val healthReportRef: Ref[F, WorkerHealth],
  stop: F[Unit]
) {

  // Getter for the last healthcheck
  val healthReport: F[WorkerHealth] = healthReportRef.get

}

object Worker extends LazyLogging {

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

  /**
   * Makes a single worker that runs once resource is in use
   *
   * @param params Worker's running params
   * @param healthCheckConfig see [[HealthCheckConfig]]
   * @param onStop A callback to launch when this worker is stopped
   * @param sttpBackend Sttp Backend to launch HTTP healthchecks and RPC endpoints
   * @return the [[Worker]] instance
   */
  def make[F[_]: Concurrent: ContextShift: Timer](
    params: WorkerParams,
    healthCheckConfig: HealthCheckConfig,
    onStop: F[Unit]
  )(
    implicit sttpBackend: SttpBackend[F, Nothing]
  ): Resource[F, Worker[F]] =
    for {
      healthReportRef ← MakeResource.refOf[F, WorkerHealth](
        WorkerNotYetLaunched(StoppedWorkerInfo(params.currentWorker))
      )

      rpc ← TendermintRpc[F](params)

      container ← DockerIO.run[F](params.dockerCommand)

      healthChecks = healthCheckStream(container, params, healthCheckConfig, rpc)

      // Runs health checker, wrapped with resource:
      // health check will be stopped when the resource is released.
      _ ← MakeResource
        .concurrentStream[F](healthChecks.evalTap(healthReportRef.set), name = s"healthChecks-${params.appId}")

      control = ControlRpc[F]()

    } yield new Worker[F](params, rpc, control, healthReportRef, onStop)

}
