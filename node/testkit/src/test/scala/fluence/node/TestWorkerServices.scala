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

package fluence.node

import cats.Applicative
import cats.effect.Timer
import cats.syntax.applicative._
import fluence.effects.docker.DockerContainerStopped
import fluence.effects.tendermint.rpc.http.TendermintHttpRpc
import fluence.effects.tendermint.rpc.websocket.TendermintWebsocketRpc
import fluence.node.workers.status.{HttpCheckNotPerformed, ServiceStatus, WorkerStatus}
import fluence.node.workers.subscription.ResponseSubscriber
import fluence.node.workers.{WorkerBlockManifests, WorkerServices}
import fluence.statemachine.client.ControlRpc

import scala.concurrent.duration.FiniteDuration
import scala.language.higherKinds

object TestWorkerServices {

  def workerServiceTestRequestResponse[F[_]: Applicative: Timer](
    rpc: TendermintHttpRpc[F],
    wrpc: TendermintWebsocketRpc[F],
    requestResponderImpl: ResponseSubscriber[F]
  )(appId: Long): WorkerServices[F] = {
    new WorkerServices[F] {
      override def tendermintRpc: TendermintHttpRpc[F] = rpc
      override def tendermintWRpc: TendermintWebsocketRpc[F] = wrpc

      override def control: ControlRpc[F] = throw new NotImplementedError("def control")

      override def status(timeout: FiniteDuration): F[WorkerStatus] =
        WorkerStatus(
          isHealthy = true,
          appId = appId,
          ServiceStatus(Left(DockerContainerStopped(0)), HttpCheckNotPerformed("dumb")),
          ServiceStatus(Left(DockerContainerStopped(0)), HttpCheckNotPerformed("dumb"))
        ).pure[F]

      override def blockManifests: WorkerBlockManifests[F] = throw new NotImplementedError("def blockManifest")

      override def responseSubscriber: ResponseSubscriber[F] = requestResponderImpl
    }
  }
}
