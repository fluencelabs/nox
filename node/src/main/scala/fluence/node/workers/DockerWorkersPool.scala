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
import cats.{Applicative, Parallel}
import cats.syntax.functor._
import cats.syntax.flatMap._
import cats.syntax.applicative._
import cats.effect.{Concurrent, ContextShift, Timer}
import cats.effect.concurrent.{Deferred, Ref}
import com.softwaremill.sttp.SttpBackend
import fluence.node.workers.health.HealthCheckConfig
import slogging.LazyLogging
import cats.instances.list._
import cats.syntax.applicativeError._
import cats.syntax.flatMap._
import cats.syntax.functor._

import scala.language.higherKinds

/**
 * Wraps several [[Worker]]s in a pool, providing running and monitoring functionality.
 *
 * @param workers a storage for running [[Worker]]s, indexed by appIds
 * @param healthCheckConfig see [[HealthCheckConfig]]
 */
class DockerWorkersPool[F[_]: ContextShift: Timer](
  workers: Ref[F, Map[Long, Worker[F]]],
  healthCheckConfig: HealthCheckConfig
)(
  implicit sttpBackend: SttpBackend[F, Nothing],
  F: Concurrent[F]
) extends WorkersPool[F] with LazyLogging {

  /**
   * Returns true if the worker is in the pool and healthy, and false otherwise. Also returns worker instance.
   */
  private def checkWorkerHealthy(appId: Long): F[(Boolean, Option[Worker[F]])] = {
    for {
      map <- workers.get
      oldWorker = map.get(appId)
      healthy <- oldWorker match {
        case None => F.pure(false)
        case Some(worker) => worker.healthReport.map(_.isHealthy)
      }
    } yield (healthy, oldWorker)
  }

  private def runWorker(params: WorkerParams): F[Unit] =
    for {
      deferred ← Deferred[F, Unit]
      _ <- Concurrent[F].start(
        Worker
          .run[F](params, healthCheckConfig, deferred.complete(()))
          .use(
            worker ⇒
              for {
                _ ← workers.update(_.updated(params.appId, worker))
                _ ← deferred.get
                _ = logger.info(s"Removing worker from a pool: $params")
                _ ← workers.update(_ - params.appId)
              } yield ()
          )
      )

    } yield ()

  /**
   * Runs a new [[Worker]] in the pool.
   *
   * @param params see [[WorkerParams]]
   * @return F that resolves with true when worker is registered; it might be not running yet. If it was registered before, F resolves with false
   */
  override def run(params: WorkerParams): F[WorkersPool.RunResult] =
    checkWorkerHealthy(params.appId).flatMap {
      case (false, oldWorker) ⇒
        for {
          // stop an old worker
          _ <- oldWorker.fold(().pure[F])(stop)
          _ ← runWorker(params)
        } yield
          if (oldWorker.isDefined) WorkersPool.Restarted
          else WorkersPool.Ran

      case (true, oldWorker) ⇒
        logger.info(s"Worker for app ${params.appId} was already ran as $oldWorker")
        Applicative[F].pure(WorkersPool.AlreadyRunning)
    }

  private def stop(worker: Worker[F]): F[Unit] =
    worker.stop.attempt.map(stopped ⇒ logger.info(s"Stopped: ${worker.params} => $stopped"))

  /**
   * Stops all the registered workers. They should unregister themselves.
   *
   * @param P Parallel instance is required as all workers are stopped concurrently
   * @return F that resolves when all workers are stopped
   */
  def stopAll[G[_]]()(implicit P: Parallel[F, G]): F[Unit] =
    for {
      workersMap ← workers.get
      workers = workersMap.values.toList

      stops ← Parallel.parTraverse(workers)(_.stop.attempt)
    } yield logger.info(s"Stopped: ${workers.map(_.params) zip stops}")

  override def get(appId: Long): F[Option[Worker[F]]] =
    workers.get.map(_.get(appId))

  override val getAll: fs2.Stream[F, Worker[F]] =
    fs2.Stream.eval(workers.get).map(_.values).flatMap(ws ⇒ fs2.Stream(ws.toSeq: _*))

}
