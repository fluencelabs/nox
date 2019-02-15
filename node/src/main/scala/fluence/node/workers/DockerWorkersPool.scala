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
import cats.syntax.applicative._
import cats.effect._
import cats.effect.concurrent.{Deferred, Ref}
import com.softwaremill.sttp.SttpBackend
import slogging.LazyLogging
import cats.instances.list._
import cats.syntax.applicativeError._
import cats.syntax.flatMap._
import cats.syntax.apply._
import cats.syntax.functor._

import scala.language.higherKinds

/**
 * Wraps several [[Worker]]s in a pool, providing running and monitoring functionality.
 *
 * @param workers a storage for running [[Worker]]s, indexed by appIds
 */
class DockerWorkersPool[F[_]: ContextShift](
  workers: Ref[F, Map[Long, Worker[F]]],
  waitStopped: Ref[F, Map[Long, F[Unit]]]
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
        case Some(worker) => worker.status.map(_.isHealthy)
      }
    } yield (healthy, oldWorker)
  }

  /**
   * Runs a worker concurrently, registers it in `workers` map
   *
   * @param params Worker's description
   * @param stopTimeout Timeout in seconds to allow graceful stopping of running containers.
   *                    It might take up to 2*`stopTimeout` seconds to gracefully stop the worker, as 2 containers involved.
   * @return Unit; no failures are expected
   */
  private def runWorker(params: WorkerParams, stopTimeout: Int = 5): F[Unit] =
    for {
      // Fix for the almost impossible case when a command to run a new worker for an app id is received when
      // another worker with the same app id is being stopped
      _ ← waitStopped.get.flatMap(_.getOrElse(params.appId, F.unit))

      // Remember that Deferred is like a non-blocking promise, but generalized for F[_]
      // When completed, releases the used worker
      stopWorkerDef ← Deferred[F, Unit]

      // Used to pass the worker's fiber inside worker's callbacks, which are defined before we have the fiber
      runningWorkerFiberDef ← Deferred[F, Fiber[F, Unit]]

      // Fiber for the worker, needs to be joined to ensure worker cleanup process is completed
      runningWorkerFiber ← Concurrent[F].start(
        DockerWorker
          .make[F](
            params,
            // onStop is called externally, when one wants to stop the worker
            onStop = for {
              // Release the worker resource, triggering resource cleanup
              _ ← stopWorkerDef.complete(())
              // Get the worker's fiber
              fiber ← runningWorkerFiberDef.get
              // Wait for the worker resource to be released and cleaned up
              _ ← fiber.join

              // Remove stopper, so that we don't need to wait for this worker to stop on stopAll
              _ ← waitStopped.update(_ - params.appId)
            } yield logger.info(s"Worker's Fiber joined: $params"),
            stopTimeout
          )
          .use(
            worker ⇒
              for {
                // Register worker in the pool
                _ ← workers.update(_.updated(params.appId, worker)) *>
                  waitStopped.update(_.updated(params.appId, runningWorkerFiberDef.get.flatMap(_.join)))

                // Worker is being used as a resource until this Deferred is resolved
                _ ← stopWorkerDef.get

                _ = logger.info(s"Removing worker from the pool: $params")
                // Remove worker from the pool, so that worker's services cannot be used
                _ ← workers.update(_ - params.appId)
              } yield logger.info(s"Releasing the worker resource: $params")
          )
          .map(_ ⇒ logger.debug(s"Worker removed from pool: $params"))
      )

      // Pass the worker fiber to the Deferred
      _ ← runningWorkerFiberDef.complete(runningWorkerFiber)

    } yield ()

  /**
   * Runs a new [[Worker]] in the pool.
   *
   * @param params see [[WorkerParams]]
   * @return F that resolves with true when worker is registered; it might be not running yet. If it was registered before, F resolves with false
   */
  override def run(params: WorkerParams): F[WorkersPool.RunResult] =
    checkWorkerHealthy(params.appId)
      .flatMap[WorkersPool.RunResult] {
        case (false, oldWorker) ⇒
          for {
            // stop the old worker
            _ ← oldWorker.fold(().pure[F])(stop)
            _ ← runWorker(params)
          } yield
            if (oldWorker.isDefined) WorkersPool.Restarted
            else WorkersPool.Ran

        case (true, oldWorker) ⇒
          logger.info(s"Worker for app ${params.appId} was already ran as $oldWorker")
          Applicative[F].pure(WorkersPool.AlreadyRunning)
      }
      .handleError(err ⇒ WorkersPool.RunFailed(Option(err)))

  /**
   * Try to stop a worker, tolerating possible failures
   *
   * @param worker Worker to stop
   * @return Unit, no failures are possible
   */
  private def stop(worker: Worker[F]): F[Unit] =
    worker.stop.attempt.map(stopped ⇒ logger.info(s"Stopped: ${worker.description} => $stopped"))

  /**
   * Stops all the registered workers. They should unregister themselves.
   *
   * @param P Parallel instance is required as all workers are stopped concurrently
   * @return F that resolves when all workers are stopped
   */
  def stopAll[G[_]]()(implicit P: Parallel[F, G]): F[Unit] =
    for {
      workers ← getAll

      stops ← Parallel.parTraverse(workers)(_.stop.attempt)

      // Wait for workers which are being stopped separately
      notStopped ← waitStopped.get
      _ = logger.debug(s"Having to wait for ${notStopped.size} workers to stop themselves...")

      _ ← Parallel.parTraverse_(notStopped.values.toList)(identity)
    } yield logger.info(s"Stopped: ${workers.map(_.description) zip stops}")

  /**
   * Get a Worker by its appId, if it's present
   *
   * @param appId Application id
   * @return Worker
   */
  override def get(appId: Long): F[Option[Worker[F]]] =
    workers.get.map(_.get(appId))

  /**
   * Get all known workers
   *
   * @return Up-to-date list of workers
   */
  override val getAll: F[List[Worker[F]]] =
    workers.get.map(_.values.toList)

}

object DockerWorkersPool {

  /**
   * Build a new [[DockerWorkersPool]]. All workers will be stopped when the pool is released
   */
  def make[F[_]: ContextShift, G[_]]()(
    implicit
    sttpBackend: SttpBackend[F, Nothing],
    F: Concurrent[F],
    P: Parallel[F, G]
  ): Resource[F, DockerWorkersPool[F]] =
    Resource.make {
      for {
        workers ← Ref.of[F, Map[Long, Worker[F]]](Map.empty)
        stoppers ← Ref.of[F, Map[Long, F[Unit]]](Map.empty)
      } yield new DockerWorkersPool[F](workers, stoppers)
    }(_.stopAll())
}
