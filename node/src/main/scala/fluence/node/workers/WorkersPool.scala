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
 * Wraps several [[Worker]]s in a pool, providing running and monitoring functionality.
 *
 * @param workers a storage for running [[Worker]]s
 * @param cleanups a storage for cleanup fibers to be able to "block" until [[Worker]]s are stopped and removed
 * @param healthCheckConfig see [[HealthCheckConfig]]
 */
class WorkersPool[F[_]: ContextShift: Timer](
  workers: Ref[F, Map[WorkerParams, Worker[F]]],
  cleanups: Ref[F, Map[WorkerParams, F[Unit]]],
  healthCheckConfig: HealthCheckConfig
)(
  implicit sttpBackend: SttpBackend[F, Nothing],
  F: Concurrent[F]
) extends LazyLogging {

  /**
   * Returns true if the worker is in the pool and healthy, and false otherwise. Also returns worker instance.
   */
  private def checkWorkerHealthy(params: WorkerParams): F[(Boolean, Option[Worker[F]])] = {
    for {
      map <- workers.get
      oldWorker = map.get(params)
      healthy <- oldWorker match {
        case None => F.delay(false)
        case Some(worker) => worker.healthReport.map(_.isHealthy)
      }
    } yield (healthy, oldWorker)
  }

  /**
   * Runs a new [[Worker]] in the pool.
   *
   * @param params see [[WorkerParams]]
   * @return F that resolves with true when worker is registered; it might be not running yet. If it was registered before, F resolves with false
   */
  def run(params: WorkerParams): F[Boolean] =
    checkWorkerHealthy(params).flatMap {
      case (false, oldWorker) ⇒
        for {
          // stop an old worker
          _ <- oldWorker.map(stop).getOrElse(F.unit)
          worker <- Worker.run(params, healthCheckConfig)
          _ ← workers.update(_.updated(params, worker))
          cleanupFiber ← Concurrent[F].start(worker.fiber.join.attempt.flatMap { r ⇒
            logger.info(s"Removing worker from a pool: $worker due to $r")
            workers.update(_ - params) *> cleanups.update(_ - params)
          })
          _ ← cleanups.update(_ + (params → cleanupFiber.join))
        } yield true
      case (true, _) ⇒
        logger.info(s"Worker $params was already ran")
        false.pure[F]
    }

  def stop(worker: Worker[F]): F[Unit] =
    for {
      cleanupsMap ← cleanups.get
      _ <- worker.stop
      fiberJoin <- worker.fiber.join.attempt
      cleanupJoin <- cleanupsMap.getOrElse(worker.params, F.unit).attempt
    } yield logger.info(s"Stopped: $fiberJoin $cleanupJoin")

  /**
   * Stops all the registered workers. They should unregister themselves.
   *
   * @param P Parallel instance is required as all workers are stopped concurrently
   * @return F that resolves when all workers are stopped
   */
  def stopAll[G[_]](implicit P: Parallel[F, G]): F[Unit] =
    for {
      workersMap ← workers.get
      cleanupsMap ← cleanups.get
      workers = workersMap.values.toList

      _ ← Parallel.parTraverse(workers)(_.stop)
      fiberJoins ← Parallel.parTraverse(workers)(s ⇒ s.fiber.join.attempt.map(s.params → _))

      cleanupsJoins ← Parallel.parTraverse(cleanupsMap.toList)(_._2.attempt)
    } yield logger.info(s"Stopped: $fiberJoins $cleanupsJoins")

  /**
   * Returns a map of all currently registered workers, along with theirs health
   *
   * @param P [[Parallel]] instance is required as all workers are stopped concurrently
   */
  def healths[G[_]](implicit P: Parallel[F, G]): F[Map[WorkerParams, WorkerInfo]] =
    for {
      workersMap ← workers.get
      workersHealth ← Parallel.parTraverse(workersMap.values.toList)(s ⇒ s.healthReport.map(s.params → _))
    } yield workersHealth.toMap
}

object WorkersPool {

  /**
   * Build a new [[WorkersPool]]
   */
  def apply[F[_]: Concurrent: ContextShift: Timer](healthCheckConfig: HealthCheckConfig = HealthCheckConfig())(
    implicit sttpBackend: SttpBackend[F, Nothing]
  ): F[WorkersPool[F]] =
    for {
      workers ← Ref.of[F, Map[WorkerParams, Worker[F]]](Map.empty)
      cleanups ← Ref.of[F, Map[WorkerParams, F[Unit]]](Map.empty)
    } yield new WorkersPool[F](workers, cleanups, HealthCheckConfig())
}
