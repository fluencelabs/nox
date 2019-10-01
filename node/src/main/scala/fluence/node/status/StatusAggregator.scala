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

package fluence.node.status

import cats.Parallel
import cats.effect._
import cats.syntax.functor._
import cats.syntax.applicative._
import cats.syntax.flatMap._
import cats.instances.list._
import fluence.log.Log
import fluence.node.config.MasterConfig
import fluence.node.eth.{NodeEth, NodeEthState}
import fluence.worker.{WorkerNotAllocated, WorkerStatus, WorkersPool}
import shapeless.HList

import scala.concurrent.duration._
import scala.language.higherKinds

/**
 * The manager that able to get information about master node and all workers.
 *
 * @param config config file about a master node
 */
case class StatusAggregator[F[_]: Timer: Concurrent](
  config: MasterConfig,
  pool: WorkersPool[F, _, _ <: HList],
  nodeEth: NodeEth[F],
  startTimeMillis: Long
) {

  /**
   * Gets all state information about master node and workers.
   *
   * @return gathered information
   */
  def getStatus(statusTimeout: FiniteDuration)(implicit P: Parallel[F], log: Log[F]): F[MasterStatus] =
    for {
      currentTime ← Clock[F].monotonic(MILLISECONDS)
      workers ← pool.listAll()
      workerInfos <- Parallel
        .parTraverse(workers)(
          ctx ⇒
            ctx.worker
              .foldF(
                WorkerNotAllocated(_).pure[F].widen[WorkerStatus],
                w ⇒ w.status(statusTimeout)
              )
              .map(ctx.app.id → _)
        )
    } yield MasterStatus(
      config.endpoints.ip.getHostAddress,
      currentTime - startTimeMillis,
      workerInfos.size,
      workerInfos
    )

  /**
   * Just an expected Ethereum state -- a granular accessor
   */
  val expectedEthState: F[NodeEthState] =
    nodeEth.expectedState
}

object StatusAggregator {

  /**
   * Makes a StatusAggregator, lifted into Resource.
   *
   * @param masterConfig Master config
   */
  def make[F[_]: Timer: ContextShift: Concurrent: Log](
    masterConfig: MasterConfig,
    pool: WorkersPool[F, _, _ <: HList],
    nodeEth: NodeEth[F]
  ): Resource[F, StatusAggregator[F]] =
    Resource.liftF(
      for {
        startTimeMillis ← Clock[F].realTime(MILLISECONDS)
        _ ← Log[F].debug("Start time millis: " + startTimeMillis)
      } yield StatusAggregator(masterConfig, pool, nodeEth, startTimeMillis)
    )

}
