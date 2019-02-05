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
import cats.effect._
import cats.effect.concurrent.Ref
import cats.syntax.functor._
import com.softwaremill.sttp.SttpBackend
import fluence.node.workers.health.HealthCheckConfig

import scala.language.higherKinds

/**
 * Represents the algebra for working with a pool of workers.
 * All we need is to run them in pool and to retrieve them from pool.
 *
 * @tparam F Effect
 */
trait WorkersPool[F[_]] {

  /**
   * Run or restart a worker
   *
   * @param params Worker's description
   * @return Whether worker run or not
   */
  def run(params: WorkerParams): F[WorkersPool.RunResult]

  /**
   * Get a Worker by its appId, if it's present
   *
   * @param appId Application id
   * @return Worker
   */
  def get(appId: Long): F[Option[Worker[F]]]

  /**
   * Get all known workers
   *
   * @return Up-to-date list of workers
   */
  def getAll: F[List[Worker[F]]]
}

object WorkersPool {
  sealed trait RunResult
  case object Restarted extends RunResult
  case class RunFailed(reason: Option[Throwable] = None) extends RunResult
  case object AlreadyRunning extends RunResult
  case object Ran extends RunResult

  /**
   * Build a new [[WorkersPool]]. All workers will be stopped when the pool is released
   */
  def make[F[_]: ContextShift: Timer, G[_]](healthCheckConfig: HealthCheckConfig = HealthCheckConfig())(
    implicit
    sttpBackend: SttpBackend[F, Nothing],
    F: Concurrent[F],
    P: Parallel[F, G]
  ): Resource[F, WorkersPool[F]] =
    Resource.make {
      for {
        workers ← Ref.of[F, Map[Long, Worker[F]]](Map.empty)
      } yield new DockerWorkersPool[F](workers, HealthCheckConfig())
    }(_.stopAll()).map(p ⇒ p: WorkersPool[F])
}
