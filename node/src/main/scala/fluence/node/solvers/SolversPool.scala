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

import cats.Parallel
import cats.effect.concurrent.Ref
import cats.effect.{Concurrent, ContextShift, Timer}
import cats.instances.list._
import cats.syntax.applicative._
import cats.syntax.applicativeError._
import cats.syntax.apply._
import cats.syntax.flatMap._
import cats.syntax.functor._
import com.softwaremill.sttp.SttpBackend
import slogging.LazyLogging

import scala.language.higherKinds

/**
 * Wraps several Solvers in a pool, providing running and monitoring functionality.
 *
 * @param solvers a storage for running solvers
 * @param cleanups a storage for cleanup fibers to be able to "block" until solvers are stopped and removed
 * @param healthCheckConfig see [[HealthCheckConfig]]
 */
class SolversPool[F[_]: Concurrent: ContextShift: Timer](
  solvers: Ref[F, Set[Solver[F]]],
  cleanups: Ref[F, Map[SolverParams, F[Unit]]],
  healthCheckConfig: HealthCheckConfig
)(
  implicit sttpBackend: SttpBackend[F, Nothing]
) extends LazyLogging {

  /**
   * Runs a new solver in the pool.
   *
   * @param params see [[SolverParams]]
   * @return F that resolves with true when solver is registered; it might be not running yet. If it was registered before, F resolves with false
   */
  def run(params: SolverParams): F[Boolean] =
    solvers.get.map(_.exists(_.params == params)).flatMap {
      case false ⇒
        for {
          solver <- Solver.run(params, healthCheckConfig)
          _ ← solvers.update(_ + solver)
          cleanupFiber ← Concurrent[F].start(solver.fiber.join.attempt.flatMap { r ⇒
            logger.info(s"Removing solver from a pool: $solver due to $r")
            solvers.update(_ - solver) *> cleanups.update(_ - params)
          })
          _ ← cleanups.update(_ + (params → cleanupFiber.join))
        } yield true
      case true ⇒
        logger.info(s"Solver $params was already ran")
        false.pure[F]
    }

  /**
   * Stops all the registered solvers. They should unregister themselves.
   *
   * @param P Parallel instance is required as all solvers are stopped concurrently
   * @return F that resolves when all solvers are stopped
   */
  def stopAll[G[_]](implicit P: Parallel[F, G]): F[Unit] =
    for {
      ss ← solvers.get
      cs ← cleanups.get

      _ ← Parallel.parTraverse(ss.toList)(_.stop)
      fiberJoins ← Parallel.parTraverse(ss.toList)(s ⇒ s.fiber.join.attempt.map(s.params → _))

      cleanupsJoins ← Parallel.parTraverse(cs.toList)(_._2.attempt)
    } yield logger.info(s"Stopped: $fiberJoins $cleanupsJoins")

  /**
   * Returns a map of all currently registered solvers, along with theirs health
   *
   * @param P Parallel instance is required as all solvers are stopped concurrently
   */
  def healths[G[_]](implicit P: Parallel[F, G]): F[Map[SolverParams, SolverHealth]] =
    for {
      ss ← solvers.get
      sh ← Parallel.parTraverse(ss.toList)(s ⇒ s.healthReport.map(s.params → _))
    } yield sh.toMap

}

object SolversPool {

  /**
   * Build a new SolversPool
   */
  def apply[F[_]: Concurrent: ContextShift: Timer](healthCheckConfig: HealthCheckConfig = HealthCheckConfig())(
    implicit sttpBackend: SttpBackend[F, Nothing]
  ): F[SolversPool[F]] =
    for {
      solvers ← Ref.of[F, Set[Solver[F]]](Set.empty)
      cleanups ← Ref.of[F, Map[SolverParams, F[Unit]]](Map.empty)
    } yield new SolversPool[F](solvers, cleanups, HealthCheckConfig())
}
