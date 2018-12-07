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

package fluence.node.solvers

import cats.Applicative
import cats.effect._
import cats.effect.concurrent.{Deferred, Ref}
import cats.syntax.functor._
import cats.syntax.flatMap._
import cats.syntax.either._
import cats.syntax.applicativeError._
import com.softwaremill.sttp._
import com.softwaremill.sttp.circe._
import fluence.node.docker.DockerIO
import slogging.LazyLogging

import scala.language.higherKinds

/**
 * Single running solver's datatype
 *
 * @param healthReportRef a reference to the last healthcheck, updated every time a new healthcheck is being made
 * @param stop stops the solver, should be launched only once
 * @param fiber a fiber for the cuncurrently launched stream of healthchecks for this solver
 * @tparam F the effect
 */
case class Solver[F[_]](
  params: SolverParams,
  private val healthReportRef: Ref[F, SolverHealth],
  stop: F[Unit],
  fiber: Fiber[F, Unit]
) {

  // Getter for the last healthcheck
  val healthReport: F[SolverHealth] = healthReportRef.get

}

object Solver extends LazyLogging {

  import SolverResponse._

  /**
   * Gets health state from a solver via HTTP.
   *
   */
  private def getHealthState[F[_]: Concurrent: ContextShift: Timer](
    params: SolverParams,
    httpPath: String,
    uptime: Long
  )(implicit sttpBackend: SttpBackend[F, Nothing]): F[SolverHealth] = {

    val url = uri"http://${params.clusterData.rpcHost}:${params.rpcPort}/$httpPath"

    // As container is running, perform a custom healthcheck: request a HTTP endpoint inside the container
    logger.debug(
      s"Running HTTP healthcheck $params: $url"
    )
    sttp
      .get(url)
      .response(asJson[SolverResponse])
      .send()
      .attempt
      // converting Either[Throwable, Response[Either[DeserializationError[circe.Error], SolverResponse]]]
      // to Either[Throwable, SolverResponse]
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
          val info = RunningSolverInfo.fromParams(params, tendermintInfo)
          SolverRunning(uptime, info)
        case Left(err) ⇒
          logger.error("Solver HTTP check failed: " + err.getLocalizedMessage, err)
          SolverHttpCheckFailed(StoppedSolverInfo(params), err)
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
    params: SolverParams,
    healthReportRef: Ref[F, SolverHealth],
    stop: Deferred[F, Either[Throwable, Unit]],
    healthcheck: HealthCheckConfig
  )(implicit sttpBackend: SttpBackend[F, Nothing]): F[Unit] =
    DockerIO
      .run[F](params.dockerCommand)
      .through(
        // Check that container is running every healthcheck.period
        DockerIO.check[F](healthcheck.period)
      )
      .evalMap[F, SolverHealth] {
        case (uptime, true) ⇒
          getHealthState(params, healthcheck.httpPath, uptime)
        case (_, false) ⇒
          logger.error(s"HTTP healthcheck $params, as container is not running")
          Applicative[F].pure(SolverContainerNotRunning(StoppedSolverInfo(params)))
      }
      .evalTap(healthReportRef.set)
      .interruptWhen(stop)
      .sliding(healthcheck.slide)
      .evalTap[F] {
        case q if q.count(!_.isHealthy) > healthcheck.failOn ⇒
          // TODO: if we had container launched previously, but then http checks became failing, we should try to restart the container
          logger.error("Too many healthcheck failures.")
          Applicative[F].unit
        case _ ⇒ Applicative[F].unit
      }
      .compile
      .drain

  /**
   * Runs a single solver
   *
   * @param params Solver's running params
   * @param healthcheck see [[HealthCheckConfig]]
   * @param sttpBackend Sttp Backend to launch HTTP healthchecks
   * @return the solver instance
   */
  def run[F[_]: Concurrent: ContextShift: Timer](params: SolverParams, healthcheck: HealthCheckConfig)(
    implicit sttpBackend: SttpBackend[F, Nothing]
  ): F[Solver[F]] =
    for {
      healthReportRef ← Ref.of[F, SolverHealth](SolverNotYetLaunched(StoppedSolverInfo(params)))
      stop ← Deferred[F, Either[Throwable, Unit]]

      fiber ← Concurrent[F].start(runHealthCheck(params, healthReportRef, stop, healthcheck))
    } yield Solver[F](params, healthReportRef, stop.complete(Right(())), fiber)

}
