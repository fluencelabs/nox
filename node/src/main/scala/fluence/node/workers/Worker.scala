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
import cats.syntax.applicativeError._
import cats.syntax.apply._
import cats.syntax.either._
import cats.syntax.flatMap._
import cats.syntax.functor._
import com.softwaremill.sttp._
import com.softwaremill.sttp.circe._
import fluence.node.docker.DockerIO
import slogging.LazyLogging

import scala.language.higherKinds

/**
 * Single running worker's datatype
 *
 * @param params this worker's description
 * @param rpc RPC endpoints for the worker
 * @param healthReportRef a reference to the last healthcheck, updated every time a new healthcheck is being made
 * @param stop stops the worker, should be launched only once
 * @tparam F the effect
 */
case class Worker[F[_]] private (
  params: WorkerParams,
  rpc: WorkerRpc[F],
  private val healthReportRef: Ref[F, WorkerHealth],
  stop: F[Unit]
) {

  // Getter for the last healthcheck
  val healthReport: F[WorkerHealth] = healthReportRef.get

}

object Worker extends LazyLogging {

  import WorkerResponse._

  /**
   * Gets health state from a worker via HTTP.
   *
   */
  private def getHealthState[F[_]: Concurrent: ContextShift: Timer](
    params: WorkerParams,
    statusPath: String,
    uptime: Long
  )(implicit sttpBackend: SttpBackend[F, Nothing]): F[WorkerHealth] = {

    val url = uri"http://${params.currentWorker.ip.getHostAddress}:${params.currentWorker.rpcPort}/$statusPath"

    // As container is running, perform a custom healthcheck: request a HTTP endpoint inside the container
    logger.debug(
      s"Running HTTP healthcheck $params: $url"
    )
    sttp
      .get(url)
      .response(asJson[WorkerResponse])
      .send()
      .attempt
      // converting Either[Throwable, Response[Either[DeserializationError[circe.Error], WorkerResponse]]]
      // to Either[Throwable, WorkerResponse]
      .map(
        _.flatMap(
          _.body
            .leftMap(new Exception(_))
            .flatMap(_.leftMap(_.error))
        )
      )
      .map {
        case Right(status) ⇒
          val tendermintInfo = status.result
          val info = RunningWorkerInfo(params, tendermintInfo)
          WorkerRunning(uptime, info)
        case Left(err) ⇒
          logger.error("Worker HTTP check failed: " + err.getLocalizedMessage, err)
          WorkerHttpCheckFailed(StoppedWorkerInfo(params.currentWorker), err)
      }
      .map { health ⇒
        logger.debug(s"HTTP health is: $health")
        health
      }
  }

  /**
   * Runs health checker.
   */
  private def runHealthCheck[F[_]: Concurrent: ContextShift: Timer](
    params: WorkerParams,
    healthReportRef: Ref[F, WorkerHealth],
    stop: Deferred[F, Either[Throwable, Unit]],
    healthcheck: HealthCheckConfig
  )(implicit sttpBackend: SttpBackend[F, Nothing]): F[Unit] =
    DockerIO
      .run[F](params.dockerCommand)
      .through(
        // Check that container is running every healthcheck.period
        DockerIO.check[F](healthcheck.period)
      )
      .evalMap[F, WorkerHealth] {
        case (uptime, true) ⇒
          getHealthState(params, healthcheck.httpPath, uptime)
        case (_, false) ⇒
          logger.error(s"Healthcheck is failing for worker: $params")
          Applicative[F].pure(WorkerContainerNotRunning(StoppedWorkerInfo(params.currentWorker)))
      }
      .evalTap(healthReportRef.set)
      .interruptWhen(stop)
      .sliding(healthcheck.slide)
      .compile
      .drain

  /**
   * Runs a single worker
   *
   * @param params Worker's running params
   * @param healthcheck see [[HealthCheckConfig]]
   * @param onStop A callback to launch when this worker is stopped
   * @param sttpBackend Sttp Backend to launch HTTP healthchecks and RPC endpoints
   * @return the [[Worker]] instance
   */
  def run[F[_]: Concurrent: ContextShift: Timer](params: WorkerParams, healthcheck: HealthCheckConfig, onStop: F[Unit])(
    implicit sttpBackend: SttpBackend[F, Nothing]
  ): F[Worker[F]] =
    for {
      healthReportRef ← Ref.of[F, WorkerHealth](
        WorkerNotYetLaunched(StoppedWorkerInfo(params.currentWorker))
      )
      stop ← Deferred[F, Either[Throwable, Unit]]

      fiber ← Concurrent[F].start(runHealthCheck(params, healthReportRef, stop, healthcheck))

      rpc ← WorkerRpc[F](params)

    } yield new Worker[F](params, rpc, healthReportRef, stop.complete(Right(())) *> rpc.stop *> fiber.join *> onStop)

}
