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

import cats.Monad
import cats.effect.Resource
import cats.syntax.flatMap._
import fluence.effects.docker.{DockerContainer, DockerIO, DockerNetwork}
import fluence.log.Log
import fluence.node.config.DockerConfig
import fluence.worker.eth.EthApp

import scala.concurrent.duration.FiniteDuration
import scala.language.higherKinds

case class WorkerDocker private[workers] (
  network: DockerNetwork,
  masterContainerId: Option[String],
  stopTimeout: FiniteDuration,
  logLevel: Log.Level,
  producer: WorkerDocker.Component,
  machine: WorkerDocker.Component
)

object WorkerDocker {
  case class Component(
    name: String,
    docker: DockerConfig
  )

  def apply[F[_]: Monad: DockerIO: Log](
    app: EthApp,
    masterNodeContainerId: Option[String],
    producerDocker: DockerConfig,
    machineDocker: DockerConfig,
    stopTimeout: FiniteDuration,
    logLevel: Log.Level
  ): Resource[F, WorkerDocker] = {
    val networkName = s"app_${app.id}_${app.cluster.currentWorker.index}"
    val machineName = s"sm_${app.id}_${app.cluster.currentWorker.index}"
    val producerName = s"bp_${app.id}_${app.cluster.currentWorker.index}"

    DockerNetwork
      .make(networkName)
      .flatTap(
        network ⇒
          masterNodeContainerId
            .map(DockerContainer(_, None))
            .fold(Resource.pure(()))(DockerNetwork.join(_, network))
      )
      .map(
        network ⇒
          WorkerDocker(
            network,
            masterNodeContainerId,
            stopTimeout,
            logLevel,
            Component(producerName, producerDocker),
            Component(machineName, machineDocker)
        )
      )
  }
}
