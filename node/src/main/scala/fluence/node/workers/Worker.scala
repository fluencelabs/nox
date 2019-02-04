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
import cats.effect.concurrent.{Deferred, Ref}
import cats.syntax.functor._
import cats.syntax.apply._
import cats.syntax.applicativeError._
import cats.syntax.flatMap._
import com.softwaremill.sttp._
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
      .evalMap(st ⇒ Timer[F].clock.realTime(MILLISECONDS).map(now ⇒ (now - st.startedAt) → st.isRunning))
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
   * Runs health checker, wrapped with resource:
   * health check will be stopped when the resource is released.
   *
   * @param healthReportRef Ref for the last [[WorkerHealth]], updated periodically
   * @param healthCheck Stream of [[WorkerHealth]]'s, updates the given ref
   * @return Resource that will stop healthchecks stram once released
   */
  private def runHealthCheck[F[_]: Concurrent: ContextShift: Timer](
    healthReportRef: Ref[F, WorkerHealth],
    healthCheck: fs2.Stream[F, WorkerHealth],
  ): Resource[F, Unit] =
    Resource
      .make(
        // We use Deferred to stop the stream when resource is released
        Deferred[F, Either[Throwable, Unit]].flatMap(
          stop ⇒
            // Run possibly infinite stream concurrently, get the fiber to join later
            Concurrent[F]
              .start(
                healthCheck
                  .evalTap(healthReportRef.set)
                  .interruptWhen(stop)
                  // TODO .sliding(healthcheck.slide) -- currently we have no behavior definition for failed healthcheck
                  .compile
                  .drain
              )
              .map(stop → _)
        )
      ) {
        case (stop, fiber) ⇒
          logger.debug("Trying to stop Worker's healthcheck stream...")
          stop.complete(Right(())) *> fiber.join.attempt.map {
            case Right(_) ⇒
              logger.debug("Worker's healthcheck stream stopped")
            case Left(err) ⇒
              logger.warn(s"Failed to stop Worker's healthcheck stream: $err", err)
          }
      }
      .void

  /**
   * Runs a single worker
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
      healthReportRef ← Resource.liftF(
        Ref.of[F, WorkerHealth](
          WorkerNotYetLaunched(StoppedWorkerInfo(params.currentWorker))
        )
      )

      rpc ← TendermintRpc[F](params)

      container ← DockerIO.run[F](params.dockerCommand)

      healthChecks = healthCheckStream(container, params, healthCheckConfig, rpc)

      _ ← runHealthCheck(healthReportRef, healthChecks)

      control = ControlRpc[F]()

    } yield new Worker[F](params, rpc, control, healthReportRef, onStop)

}
