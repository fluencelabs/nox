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

import cats.{Monad, Traverse}
import cats.effect._
import cats.syntax.functor._
import cats.syntax.flatMap._
import cats.instances.list._
import fluence.node.MasterNode
import fluence.node.config.MasterConfig
import slogging.LazyLogging

import scala.concurrent.duration._
import scala.language.higherKinds

/**
 * The manager that able to get information about master node and all workers.
 *
 * @param config config file about a master node
 * @param masterNode initialized master node
 */
case class StatusAggregator[F[_]: Monad: Clock](
  config: MasterConfig,
  masterNode: MasterNode[F],
  startTimeMillis: Long,
  statusTimeout: FiniteDuration
) {

  /**
   * Gets all state information about master node and workers.
   * @return gathered information
   */
  val getStatus: F[MasterStatus] = for {
    currentTime ← Clock[F].monotonic(MILLISECONDS)
    workers ← masterNode.pool.getAll
    workerInfos ← Traverse[List].traverse(workers)(_.withServices(identity)(_.status(statusTimeout)))
    ethState ← masterNode.nodeEth.expectedState
  } yield
    MasterStatus(
      config.endpoints.ip.getHostName,
      currentTime - startTimeMillis,
      masterNode.nodeConfig,
      workerInfos.size,
      workerInfos,
      config,
      ethState
    )
}

object StatusAggregator extends LazyLogging {

  /**
   * Makes a StatusAggregato9r, lifted into Resource.
   *
   * @param masterConfig Master config
   * @param masterNode Master node to fetch status from
   */
  def make[F[_]: Timer: ContextShift: Monad](
    masterConfig: MasterConfig,
    masterNode: MasterNode[F],
    statusTimeout: FiniteDuration = 5.seconds
  ): Resource[F, StatusAggregator[F]] =
    Resource.liftF(
      for {
        startTimeMillis ← Clock[F].realTime(MILLISECONDS)
        _ = logger.debug("Start time millis: " + startTimeMillis)
      } yield StatusAggregator(masterConfig, masterNode, startTimeMillis, statusTimeout)
    )

}
