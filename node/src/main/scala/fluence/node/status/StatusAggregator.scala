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

import cats.Traverse
import cats.effect._
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
case class StatusAggregator(config: MasterConfig, masterNode: MasterNode[IO], startTimeMillis: Long)(
  implicit clock: Clock[IO]
) {

  /**
   * Gets all state information about master node and workers.
   * @return gathered information
   */
  val getStatus: IO[MasterStatus] = for {
    currentTime ← clock.monotonic(MILLISECONDS)
    workers ← masterNode.pool.getAll
    workerInfos ← Traverse[List].traverse(workers)(_.status)
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

  def make(masterConfig: MasterConfig, masterNode: MasterNode[IO])(
    implicit cs: ContextShift[IO],
    timer: Timer[IO]
  ): Resource[IO, StatusAggregator] =
    Resource.liftF(for {
      startTimeMillis ← timer.clock.realTime(MILLISECONDS)
      _ = logger.debug("Start time millis: " + startTimeMillis)
    } yield StatusAggregator(masterConfig, masterNode, startTimeMillis))

}
