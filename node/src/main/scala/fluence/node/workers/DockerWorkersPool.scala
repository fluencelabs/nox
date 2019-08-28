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

import java.nio.ByteBuffer
import java.nio.file.Path

import cats.data.EitherT
import cats.effect._
import cats.effect.concurrent.{Deferred, Ref}
import cats.instances.list._
import cats.syntax.applicative._
import cats.syntax.applicativeError._
import cats.syntax.apply._
import cats.syntax.compose._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.{Applicative, Apply, Parallel}
import com.softwaremill.sttp.SttpBackend
import fluence.codec.PureCodec
import fluence.effects.docker.DockerIO
import fluence.effects.kvstore.RocksDBStore
import fluence.effects.receipt.storage.ReceiptStorage
import fluence.effects.tendermint.rpc.websocket.WebsocketConfig
import fluence.effects.{Backoff, EffectError}
import fluence.log.Log
import fluence.log.LogLevel.LogLevel
import fluence.node.MakeResource
import fluence.node.workers.tendermint.block.BlockUploading

import scala.concurrent.duration._
import scala.language.higherKinds

/**
 * Wraps several [[Worker]]s in a pool, providing running and monitoring functionality.
 *
 * @param workers a storage for running [[Worker]]s, indexed by appIds
 */
class DockerWorkersPool[F[_]: DockerIO: Timer: ContextShift, G[_]](
  ports: WorkersPorts[F],
  workers: Ref[F, Map[Long, Worker[F]]],
  logLevel: LogLevel,
  // TODO: it's not OK to have blockUploading here, it should be moved somewhere else
  blockUploading: BlockUploading[F],
  appReceiptStorage: Long ⇒ Resource[F, ReceiptStorage[F]],
  websocketConfig: WebsocketConfig,
  healthyWorkerTimeout: FiniteDuration = 1.second,
  stopTimeoutSeconds: Int = 5
)(
  implicit sttpBackend: SttpBackend[EitherT[F, Throwable, ?], Nothing],
  F: ConcurrentEffect[F],
  P: Parallel[F, G],
  backoff: Backoff[EffectError]
) extends WorkersPool[F] {

  /**
   * Returns true if the worker is in the pool and healthy, and false otherwise. Also returns worker instance.
   */
  private def checkWorkerHealthy(appId: Long): F[(Boolean, Option[Worker[F]])] = {
    for {
      map <- workers.get
      oldWorker = map.get(appId)
      healthy <- oldWorker match {
        case None         => F.pure(false)
        case Some(worker) => worker.isHealthy(healthyWorkerTimeout)
      }
    } yield (healthy, oldWorker)
  }

  /**
   * For the given Worker, registers it in the pool on acquire and removes on release
   *
   * @param worker Worker to register in the pool
   */
  private def registerWorker(worker: Worker[F])(implicit log: Log[F]): Resource[F, Unit] =
    Resource
      .make(
        workers.update(_ + (worker.appId -> worker)) *>
          log.info(s"Added worker ($worker) to the pool")
      )(
        _ ⇒
          workers.update(_ - worker.appId) *>
            log.info(s"Removing worker ($worker) from the pool")
      )
      .void

  /**
   * Prepares the worker resource with all the necessary bindings and lifecycle events
   *
   * @param onStop Should release the use of this resource
   * @param params Prepare WorkerParams; could be an expensive operation
   * @param p2pPort P2p port
   * @param stopTimeout Docker stop timeout
   * @return Worker resource to be used
   */
  private def workerResource(
    onStop: F[Unit],
    params: F[WorkerParams],
    p2pPort: Short,
    stopTimeout: Int,
    receiptStorage: Resource[F, ReceiptStorage[F]]
  )(implicit log: Log[F]): Resource[F, Worker[F]] =
    for {
      // Order events in the Worker context
      exec ← MakeResource.orderedEffects[F]

      // Prepare WorkerParams in the Worker context
      ps ← Resource.liftF(
        for {
          ds ← Deferred[F, WorkerParams]
          _ ← exec(params.flatMap(ds.complete))
          p ← ds.get
        } yield p
      )

      services ← DockerWorkerServices.make[F, G](ps, p2pPort, stopTimeout, logLevel, receiptStorage, websocketConfig)

      worker ← Worker.make(
        ps.appId,
        p2pPort,
        s"Worker; appId=${ps.appId} p2pPort=$p2pPort",
        services,
        exec,
        onStop = onStop,
        onRemove = ports.free(ps.appId).value.void
      )

      // Once the worker is created, run background job to connect it to all the peers
      _ ← WorkerP2pConnectivity.make(worker, ps.app.cluster.workers)

      // TODO: pass promise from WorkerP2pConnectivity to blockUploading.start
      // Start uploading tendermint blocks and send receipts to statemachine
      _ <- blockUploading.start(worker)
      _ <- worker.services.responseSubscriber.start()

      // Finally, register the worker in the pool
      _ ← registerWorker(worker)

    } yield worker

  /**
   * Runs a worker concurrently, registers it in the `workers` map
   *
   * @param params Worker's description
   * @param p2pPort Tendermint p2p port
   * @param stopTimeout Timeout in seconds to allow graceful stopping of running containers.
   *                    It might take up to 2*`stopTimeout` seconds to gracefully stop the worker, as 2 containers involved.
   * @return Unit; no failures are expected
   */
  def runWorker(
    p2pPort: Short,
    params: F[WorkerParams],
    stopTimeout: Int,
    receiptStorage: Resource[F, ReceiptStorage[F]]
  )(
    implicit log: Log[F]
  ): F[Unit] =
    MakeResource.useConcurrently[F](
      workerResource(
        _,
        params,
        p2pPort,
        stopTimeout,
        receiptStorage
      )
    )

  /**
   * Runs a new [[Worker]] in the pool.
   *
   * @param params see [[WorkerParams]]
   * @return F that resolves with true when worker is registered; it might be not running yet. If it was registered before, F resolves with false
   */
  override def run(appId: Long, params: F[WorkerParams])(implicit log: Log[F]): F[WorkersPool.RunResult] =
    // TODO worker should be responsible for restarting itself, so that we don't block here
    Apply[F]
      .product(checkWorkerHealthy(appId), ports.allocate(appId).value)
      .flatMap[WorkersPool.RunResult] {
        case ((false, oldWorker), Right(p2pPort)) ⇒
          for {
            // stop the old worker
            _ ← oldWorker.fold(().pure[F])(stop)

            _ ← runWorker(p2pPort, params, stopTimeoutSeconds, appReceiptStorage(appId))

          } yield
            if (oldWorker.isDefined) WorkersPool.Restarting
            else WorkersPool.Starting

        case ((true, oldWorker), _) ⇒
          log.info(s"Worker for app $appId was already ran as $oldWorker") as WorkersPool.AlreadyRunning

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
  private def stop(worker: Worker[F])(implicit log: Log[F]): F[Unit] =
    worker.stop.attempt >>= (stopped ⇒ log.info(s"Stopped: ${worker.description} => $stopped"))

  /**
   * Stops all the registered workers. They should unregister themselves.
   *
   * @return F that resolves when all workers are stopped
   */
  def stopAll()(implicit log: Log[F]): F[Unit] =
    for {
      workers ← getAll

      stops ← Parallel.parTraverse(workers)(_.stop.attempt)

      // TODO join fibers?

      // Wait for workers which are being stopped separately
      //notStopped ← waitStopped.get
      //_ = logger.debug(s"Having to wait for ${notStopped.size} workers to stop themselves...")

      //_ ← Parallel.parTraverse_(notStopped.values.toList)(identity)
      _ ← Log[F].info(s"Stopped: ${workers.map(_.description) zip stops}")
    } yield ()

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

  private val P2pPortsDbFolder: String = "p2p-ports-db"

  /**
   * Build a new [[DockerWorkersPool]]. All workers will be stopped when the pool is released
   */
  def make[F[_]: DockerIO: ContextShift: Timer: Log, G[_]](
    minPort: Short,
    maxPort: Short,
    rootPath: Path,
    appReceiptStorage: Long ⇒ Resource[F, ReceiptStorage[F]],
    workerLogLevel: LogLevel,
    websocketConfig: WebsocketConfig,
    blockUploading: BlockUploading[F]
  )(
    implicit
    sttpBackend: SttpBackend[EitherT[F, Throwable, ?], fs2.Stream[F, ByteBuffer]],
    F: ConcurrentEffect[F],
    P: Parallel[F, G],
    backoff: Backoff[EffectError]
  ): Resource[F, WorkersPool[F]] =
    for {
      ports ← makePorts(minPort, maxPort, rootPath)
      pool ← Resource.make {
        for {
          workers ← Ref.of[F, Map[Long, Worker[F]]](Map.empty)
        } yield new DockerWorkersPool[F, G](
          ports,
          workers,
          workerLogLevel,
          blockUploading,
          appReceiptStorage,
          websocketConfig
        )
      }(_.stopAll())
    } yield pool: WorkersPool[F]

  private def makePorts[F[_]: Concurrent: LiftIO: ContextShift: Log](
    minPort: Short,
    maxPort: Short,
    rootPath: Path
  ): Resource[F, WorkersPorts[F]] =
    for {
      _ <- Log.resource[F].debug("Making ports for a WorkersPool, first prepare RocksDBStore")

      // TODO use better serialization, check for errors
      implicit0(stringCodec: PureCodec[String, Array[Byte]]) = PureCodec
        .liftB[String, Array[Byte]](_.getBytes(), bs ⇒ new String(bs))

      implicit0(longCodec: PureCodec[Array[Byte], Long]) = PureCodec[Array[Byte], String] andThen PureCodec
        .liftB[String, Long](_.toLong, _.toString)

      implicit0(shortCodec: PureCodec[Array[Byte], Short]) = PureCodec[Array[Byte], String] andThen PureCodec
        .liftB[String, Short](_.toShort, _.toString)

      // TODO: handle exception
      path = rootPath.resolve(P2pPortsDbFolder)

      _ <- Log.resource[F].debug(s"Ports db: $path")

      store <- RocksDBStore.make[F, Long, Short](path.toString)

      ports ← WorkersPorts.make(minPort, maxPort, store)
    } yield ports
}
