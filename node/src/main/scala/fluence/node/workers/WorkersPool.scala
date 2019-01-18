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
import cats.effect.{Concurrent, ContextShift, Sync, Timer}
import cats.instances.list._
import cats.syntax.applicative._
import cats.syntax.applicativeError._
import cats.syntax.flatMap._
import cats.syntax.functor._
import com.softwaremill.sttp.SttpBackend
import org.web3j.abi.datatypes.generated.Bytes32
import slogging.LazyLogging

import scala.language.higherKinds

/**
 * Wraps several [[Worker]]s in a pool, providing running and monitoring functionality.
 *
 * @param workers a storage for running [[Worker]]s, indexed by appIds
 * @param healthCheckConfig see [[HealthCheckConfig]]
 */
class WorkersPool[F[_]: ContextShift: Timer](
  workers: Ref[F, Map[Bytes32, Worker[F]]],
  healthCheckConfig: HealthCheckConfig
)(
  implicit sttpBackend: SttpBackend[F, Nothing],
  F: Concurrent[F]
) extends LazyLogging {

  /**
   * Returns true if the worker is in the pool and healthy, and false otherwise. Also returns worker instance.
   */
  private def checkWorkerHealthy(appId: Bytes32): F[(Boolean, Option[Worker[F]])] = {
    for {
      map <- workers.get
      oldWorker = map.get(appId)
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
    checkWorkerHealthy(params.appId).flatMap {
      case (false, oldWorker) ⇒
        for {
          // stop an old worker
          _ <- oldWorker.map(stop).getOrElse(F.unit)
          worker <- Worker.run[F](params, healthCheckConfig, Sync[F].delay {
            logger.info(s"Removing worker from a pool: $params")
            workers.update(_ - params.appId)
          })
          _ ← workers.update(_.updated(params.appId, worker))
        } yield true
      case (true, oldWorker) ⇒
        logger.info(s"Worker for app ${params.appId} was already ran as $oldWorker")
        false.pure[F]
    }

  private def stop(worker: Worker[F]): F[Unit] =
    worker.stop.attempt.map(stopped ⇒ logger.info(s"Stopped: ${worker.params} => $stopped"))

  /**
   * Stops all the registered workers. They should unregister themselves.
   *
   * @param P Parallel instance is required as all workers are stopped concurrently
   * @return F that resolves when all workers are stopped
   */
  def stopAll[G[_]](implicit P: Parallel[F, G]): F[Unit] =
    for {
      workersMap ← workers.get
      workers = workersMap.values.toList

      stops ← Parallel.parTraverse(workers)(_.stop.attempt)
    } yield logger.info(s"Stopped: ${workers.map(_.params) zip stops}")

  /**
   * Returns a map of all currently registered workers, along with theirs health
   *
   * @param P [[Parallel]] instance is required as all workers are stopped concurrently
   */
  def healths[G[_]](implicit P: Parallel[F, G]): F[Map[WorkerParams, WorkerHealth]] =
    for {
      workersMap ← workers.get
      workersHealth ← Parallel.parTraverse(workersMap.values.toList)(s ⇒ s.healthReport.map(s.params → _))
    } yield workersHealth.toMap

  def stopWorkerForApp(appId: Bytes32): F[Unit] =
    for {
      map <- workers.get
      worker = map.get(appId)
      _ <- worker.map(stop).getOrElse(F.unit)
    } yield {}
}

object WorkersPool {

  /**
   * Build a new [[WorkersPool]]
   */
  def apply[F[_]: Concurrent: ContextShift: Timer](healthCheckConfig: HealthCheckConfig = HealthCheckConfig())(
    implicit sttpBackend: SttpBackend[F, Nothing]
  ): F[WorkersPool[F]] =
    for {
      workers ← Ref.of[F, Map[Bytes32, Worker[F]]](Map.empty)
    } yield new WorkersPool[F](workers, HealthCheckConfig())
}
