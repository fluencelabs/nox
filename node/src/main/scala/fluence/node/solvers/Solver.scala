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
import cats.syntax.applicativeError._
import com.softwaremill.sttp._
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
      ref ← Ref.of[F, SolverHealth](SolverNotYetLaunched)
      stop ← Deferred[F, Either[Throwable, Unit]]

      fiber ← Concurrent[F].start(
        DockerIO
          .run[F](params.dockerCommand)
          .through(
            // Check that container is running every healthcheck.period
            DockerIO.check[F](healthcheck.period)
          )
          .evalMap[F, SolverHealth] {
            case (d, true) ⇒
              // As container is running, perform a custom healthcheck: request a HTTP endpoint inside the container
              logger.debug(
                s"Running HTTP healthcheck $params: http://localhost:${params.rpcPort}/${healthcheck.httpPath}"
              )
              sttp
                .get(uri"http://localhost:${params.rpcPort}/${healthcheck.httpPath}")
                .send()
                .attempt
                .map {
                  case Right(_) ⇒ SolverRunning(d)
                  case Left(err) ⇒ SolverHttpCheckFailed(d, err)
                }
                .map { health ⇒
                  logger.debug(s"HTTP health is: $health")
                  health
                }

            case (d, false) ⇒
              logger.debug(s"HTTP healthcheck $params, as container is not running")
              Applicative[F].pure(SolverContainerNotRunning(d))
          }
          .evalTap(ref.set)
          .interruptWhen(stop)
          .sliding(healthcheck.slide)
          .evalTap[F] {
            case q if q.count(!_.isHealthy) > healthcheck.failOn ⇒
              // TODO: if we had container launched previously, but then http checks became failing, we should try to restart the container
              // Stop the stream, as there's too many failing healthchecks
              logger.debug("Too many healthcheck failures, raising an error")
              (new RuntimeException("Too many healthcheck failures"): Throwable).raiseError[F, Unit]
            case _ ⇒ Applicative[F].unit
          }
          .compile
          .drain
          .map(_ ⇒ logger.debug(s"Finished $params"))
      )
    } yield Solver[F](params, ref, stop.complete(Right(())), fiber)

}
