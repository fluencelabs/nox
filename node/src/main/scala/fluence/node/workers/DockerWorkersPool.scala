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
import java.nio.file.Path

import cats.{Applicative, Apply, Parallel}
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
import fluence.codec.PureCodec
import fluence.effects.docker.DockerIO
import fluence.effects.kvstore.RocksDBStore

import scala.language.higherKinds

/**
 * Wraps several [[WorkerServices]]s in a pool, providing running and monitoring functionality.
 *
 * @param workers a storage for running [[WorkerServices]]s, indexed by appIds
 */
class DockerWorkersPool[F[_]: DockerIO: Timer, G[_]](
  ports: WorkersPorts[F],
  workers: Ref[F, Map[Long, Worker[F]]]
)(
  implicit sttpBackend: SttpBackend[F, Nothing],
  F: Concurrent[F],
  P: Parallel[F, G]
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
        case Some(workerBus) => workerBus.isHealthy
      }
    } yield (healthy, oldWorker)
  }

  /**
   * Runs a worker concurrently, registers it in `workers` map
   *
   * @param params Worker's description
   * @param p2pPort Tendermint p2p port
   * @param stopTimeout Timeout in seconds to allow graceful stopping of running containers.
   *                    It might take up to 2*`stopTimeout` seconds to gracefully stop the worker, as 2 containers involved.
   * @return Unit; no failures are expected
   */
  def runWorker(appId: Long, p2pPort: Short, params: F[WorkerParams], stopTimeout: Int = 5): F[Unit] =
    for {

      // Remember that Deferred is like a non-blocking promise, but generalized for F[_]
      // When completed, releases the used worker
      stopServicesDef ← Deferred[F, Unit]

      // Used to pass the worker's fiber inside worker's callbacks, which are defined before we have the fiber
      runningServicesFiberDef ← Deferred[F, Fiber[F, Unit]]

      // To get worker out of its Resource, once it's ready
      servicesDef ← Deferred[F, WorkerServices[F]]

      workerDef ← Deferred[F, Worker[F]]

      worker ← Worker[F](
        appId,
        p2pPort,
        s"WorkerBus; appId=$appId p2pPort=$p2pPort",
        for {
          p ← params

          // Fiber for the worker, needs to be joined to ensure worker cleanup process is completed
          runningServicesFiber ← Concurrent[F].start(
            DockerWorkerServices
              .make[F](
                p,
                p2pPort,
                stopTimeout
              )
              .use(
                services ⇒
                  for {
                    // Register worker in the pool
                    _ ← servicesDef.complete(services)

                    bus ← workerDef.get

                    // Launch a concurrent process of fetching p2p ports from other nodes
                    // Once a port is received, register it in tendermint
                    p2pPortsFiber ← WorkerP2pConnectivity
                      .join(bus, p.app.cluster.workers.filterNot(_.index == p.currentWorker.index))

                    // Worker is being used as a resource until this Deferred is resolved
                    _ ← stopServicesDef.get

                    // If we haven't connected to some p2p host yet, stop trying
                    _ ← p2pPortsFiber.cancel

                  } yield logger.info(s"Releasing the worker resource: $params")
              )
              .map(_ ⇒ logger.debug(s"Worker removed from pool: $params"))
          )

          // Pass the worker fiber to the Deferred
          _ ← runningServicesFiberDef.complete(runningServicesFiber)

          w ← servicesDef.get

        } yield w,
        // onStop is called externally, when one wants to stop the worker
        onStop = for {
          // Release the worker resource, triggering resource cleanup
          _ ← stopServicesDef.complete(())
          // Get the worker's fiber
          fiber ← runningServicesFiberDef.get
          // Wait for the worker resource to be released and cleaned up
          _ ← fiber.join

          _ = logger.info(s"Removing worker from the pool: $params")
          // Remove worker from the pool, so that worker's services cannot be used
          _ ← workers.update(_ - appId)

        } yield logger.info(s"Worker's Fiber joined: $params"),
        onRemove = ports.free(appId).value.void
      )

      _ ← workerDef.complete(worker)

      _ ← workers.update(_ + (appId -> worker))
    } yield ()

  /**
   * Runs a new [[WorkerServices]] in the pool.
   *
   * @param params see [[WorkerParams]]
   * @return F that resolves with true when worker is registered; it might be not running yet. If it was registered before, F resolves with false
   */
  override def run(appId: Long, params: F[WorkerParams]): F[WorkersPool.RunResult] =
    /*
  TODO worker should be responsible for restarting itself, so that we don't block here
     */
    Apply[F]
      .product(checkWorkerHealthy(appId), ports.allocate(appId).value)
      .flatMap[WorkersPool.RunResult] {
        case ((false, oldWorker), Right(p2pPort)) ⇒
          for {
            // stop the old worker
            _ ← oldWorker.fold(().pure[F])(stop)

            _ ← runWorker(appId, p2pPort, params)

          } yield
            if (oldWorker.isDefined) WorkersPool.Restarting
            else WorkersPool.Starting

        case ((true, oldWorker), _) ⇒
          logger.info(s"Worker for app $appId was already ran as $oldWorker")
          Applicative[F].pure(WorkersPool.AlreadyRunning)

        // Cannot allocate port
        case (_, Left(err)) ⇒
          Applicative[F].pure(WorkersPool.RunFailed(Some(err)))
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
   * @return F that resolves when all workers are stopped
   */
  def stopAll(): F[Unit] =
    for {
      workers ← getAll

      stops ← Parallel.parTraverse(workers)(_.stop.attempt)

      // TODO join fibers?

      // Wait for workers which are being stopped separately
      //notStopped ← waitStopped.get
      //_ = logger.debug(s"Having to wait for ${notStopped.size} workers to stop themselves...")

      //_ ← Parallel.parTraverse_(notStopped.values.toList)(identity)
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

object DockerWorkersPool extends LazyLogging {

  private val P2pPortsDbFolder: String = "p2p-ports-db"

  /**
   * Build a new [[DockerWorkersPool]]. All workers will be stopped when the pool is released
   */
  def make[F[_]: DockerIO: ContextShift: Timer, G[_]](minPort: Short, maxPort: Short, rootPath: Path)(
    implicit
    sttpBackend: SttpBackend[F, Nothing],
    F: Concurrent[F],
    P: Parallel[F, G]
  ): Resource[F, WorkersPool[F]] =
    for {
      ports ← makePorts(minPort, maxPort, rootPath)
      pool ← Resource.make {
        for {
          workers ← Ref.of[F, Map[Long, Worker[F]]](Map.empty)
        } yield new DockerWorkersPool[F, G](ports, workers)
      }(_.stopAll())
    } yield pool: WorkersPool[F]

  private def makePorts[F[_]: Concurrent: LiftIO: ContextShift](
    minPort: Short,
    maxPort: Short,
    rootPath: Path
  ): Resource[F, WorkersPorts[F]] = {
    import cats.syntax.compose._

    logger.debug("Making ports for a WorkersPool, first prepare RocksDBStore")

    // TODO use better serialization, check for errors
    implicit val stringCodec: PureCodec[String, Array[Byte]] =
      PureCodec.liftB(_.getBytes(), bs ⇒ new String(bs))

    implicit val longCodec: PureCodec[Array[Byte], Long] =
      PureCodec[Array[Byte], String] andThen PureCodec
        .liftB[String, Long](_.toLong, _.toString)

    implicit val shortCodec: PureCodec[Array[Byte], Short] =
      PureCodec[Array[Byte], String] andThen PureCodec
        .liftB[String, Short](_.toShort, _.toString)

    val path = rootPath.resolve(P2pPortsDbFolder)

    logger.debug(s"Ports db: $path")

    RocksDBStore.make[F, Long, Short](path.toString)
  }.flatMap(WorkersPorts.make(minPort, maxPort, _))
}
