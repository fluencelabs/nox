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

package fluence.node

import cats.Applicative
import cats.effect._
import cats.effect.concurrent.{Deferred, Ref}
import cats.syntax.functor._
import cats.syntax.flatMap._
import cats.syntax.applicativeError._
import com.softwaremill.sttp._
import slogging.LazyLogging

import scala.concurrent.duration._
import scala.language.higherKinds

/**
 * Single running solver's datatype
 *
 * @param lastHealthCheckRef a reference to the last healthcheck, updated every time a new healthcheck is being made
 * @param stop stops the solver, should be launched only once
 * @param fiber a fiber for the cuncurrently launched stream of healthchecks for this solver
 * @tparam F the effect
 */
case class Solver[F[_]](
  params: Solver.Params,
  private val lastHealthCheckRef: Ref[F, Solver.Health],
  stop: F[Unit],
  fiber: Fiber[F, Unit]
) {

  // Getter for the last healthcheck
  val lastHealthCheck: F[Solver.Health] = lastHealthCheckRef.get

}

object Solver extends LazyLogging {
  type Health = (FiniteDuration, Either[String, Unit])

  /**
   * Configures the healthcheck process
   *
   * @param period How often to check solver's health
   * @param slide How many checks to slide over
   * @param failOn Solver will be considered dead if ''failOn'' checks within the last ''slide'' ones are failures
   * @param httpPath What to call to check for health, should not contain leading slash /
   */
  case class HealthcheckConfig(
    period: FiniteDuration = 3.seconds,
    slide: Int = 5,
    failOn: Int = 3,
    httpPath: String = ""
  )

  case class Params(rpcPort: Int) {
    def name = s"(solver of rpcPort $rpcPort)"
  }

  /**
   * [[DockerIO.run]]'s command for launching a configured solver
   *
   * @param params Solver's running params
   */
  private def command(params: Params): String = s"-p ${params.rpcPort}:80 nginx"

  /**
   * Runs a single solver
   *
   * @param params Solver's running params
   * @param healthcheck see [[HealthcheckConfig]]
   * @param sttpBackend Sttp Backend to launch HTTP healthchecks
   * @return the solver instance
   */
  def run[F[_]: Concurrent: ContextShift: Timer](params: Params, healthcheck: HealthcheckConfig)(
    implicit sttpBackend: SttpBackend[F, Nothing]
  ): F[Solver[F]] =
    for {
      ref ← Ref.of[F, Health]((0.millis, Left("Not yet launched")))
      stop ← Deferred[F, Either[Throwable, Unit]]

      fiber ← Concurrent[F].start(
        DockerIO
          .run[F](command(params))
          .through(
            // Check that container is running every 3 seconds
            DockerIO.check[F](healthcheck.period)
          )
          .evalMap[F, (FiniteDuration, Either[String, Unit])] {
            case (d, true) ⇒
              // As container is running, perform a custom healthcheck: request a HTTP endpoint inside the container
              logger.debug(s"Running HTTP healthcheck ${params.name}")
              sttp
                .get(uri"http://localhost:${params.rpcPort}/${healthcheck.httpPath}")
                .send()
                .attempt
                .map(_.left.map(_.getMessage).map(_ ⇒ ()))
                .map(d → _)
                .map { health ⇒
                  logger.debug(s"HTTP health is: $health")
                  health
                }

            case (d, false) ⇒
              logger.debug(s"HTTP healthcheck ${params.name}, as container is not running")
              Applicative[F].pure(d → Left("Container is not running"))
          }
          .evalTap(ref.set)
          .interruptWhen(stop)
          .sliding(healthcheck.slide)
          .evalTap[F] {
            case q if q.count(_._2.isLeft) > healthcheck.failOn ⇒
              // Stop the stream, as there's too many failing healthchecks
              logger.debug("Too many healthcheck failures, raising an error")
              (new RuntimeException("Too many failures"): Throwable).raiseError[F, Unit]
            case _ ⇒ Applicative[F].unit
          }
          .compile
          .drain
          .map(_ ⇒ logger.debug(s"Finished ${params.name}"))
      )
    } yield Solver[F](params, ref, stop.complete(Right(())), fiber)

}
