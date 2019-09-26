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

import cats.effect._
import cats.{Monad, Parallel}
import fluence.bp.api.{BlockProducer, DialPeers}
import fluence.bp.uploading.BlockUploading
import fluence.effects.docker._
import fluence.effects.receipt.storage.ReceiptStorage
import fluence.effects.sttp.SttpEffect
import fluence.effects.tendermint.rpc.websocket.WebsocketConfig
import fluence.effects.{Backoff, EffectError}
import fluence.log.Log
import fluence.node.workers.pool.WorkerP2pConnectivity
import fluence.node.workers.tendermint.DockerTendermint
import fluence.statemachine.api.StateMachine
import fluence.statemachine.api.command.{PeersControl, ReceiptBus}
import fluence.statemachine.docker.DockerStateMachine
import fluence.worker.WorkerStatus
import fluence.worker.responder.WorkerResponder

import scala.concurrent.duration.FiniteDuration
import scala.language.higherKinds

/**
 * Single running worker's datatype
 *
 * @param p2pPort Tendermint p2p port
 * @param appId Worker's app ID
 * @param statusCall Getter for actual Worker's status
 * @tparam F the effect
 */
case class DockerWorkerServices[F[_]] private (
  p2pPort: Short,
  appId: Long,
  producer: BlockProducer[F],
  machine: StateMachine[F],
  peersControl: PeersControl[F],
  responder: WorkerResponder[F],
  statusCall: FiniteDuration ⇒ F[WorkerStatus]
) extends WorkerServices[F] {
  override def status(timeout: FiniteDuration): F[WorkerStatus] = statusCall(timeout)
}

object DockerWorkerServices {

  private def dockerNetworkName(params: WorkerParams): String =
    s"fluence_${params.appId}_${params.currentWorker.index}"

  private def containerName(params: WorkerParams) =
    s"${params.appId}_worker_${params.currentWorker.index}"

  /**
   * Creates new docker network and connects node to that network
   *
   * @param params used for docker network name generation
   * @tparam F Effect
   * @return Resource of docker network and node connection.
   *         On release node will be disconnected, network will be removed.
   */
  private def makeNetwork[F[_]: DockerIO: Monad: Log](params: WorkerParams): Resource[F, DockerNetwork] =
    for {
      _ ← Log.resource[F].debug(s"Creating docker network ${dockerNetworkName(params)} for $params")
      network <- DockerNetwork.make(dockerNetworkName(params))
      _ <- params.masterNodeContainerId
        .map(DockerContainer(_, None))
        .fold(Resource.pure(()))(DockerNetwork.join(_, network))
    } yield network

  /**
   * Makes a single worker that runs once resource is in use
   *
   * @param params Worker's running params
   * @param p2pPort Tendermint p2p port
   * @param stopTimeout Timeout in seconds to allow graceful stopping of running containers.
   *                    It might take up to 2*`stopTimeout` seconds to gracefully stop the worker, as 2 containers involved.
   * @param logLevel Logging level passed to the worker
   * @param receiptStorage Receipt storage resource for this app
   * @return the [[WorkerServices]] instance
   */
  def make[F[_]: DockerIO: Timer: ConcurrentEffect: Parallel: Log: ContextShift: SttpEffect](
    params: WorkerParams,
    p2pPort: Short,
    stopTimeout: Int,
    logLevel: Log.Level,
    receiptStorage: Resource[F, ReceiptStorage[F]],
    blockUploading: BlockUploading[F],
    websocketConfig: WebsocketConfig
  )(
    implicit
    backoff: Backoff[EffectError]
  ): Resource[F, WorkerServices[F]] =
    for {
      // Create network, start worker & tendermint containers
      network ← makeNetwork(params)

      stateMachine ← DockerStateMachine.make[F](
        containerName(params),
        network,
        params.dockerConfig.limits,
        params.dockerConfig.image,
        logLevel,
        params.dockerConfig.environment,
        params.vmCodePath.toString,
        params.masterNodeContainerId,
        stopTimeout
      )

      producer ← DockerTendermint
        .make[F](params, p2pPort, containerName(params), network, stopTimeout, websocketConfig)

      // Once tendermint is started, run background job to connect it to all the peers
      _ ← WorkerP2pConnectivity.make(params.app.id, producer.command[DialPeers[F]], params.app.cluster.workers)

      blockManifests ← WorkerBlockManifests.make[F](receiptStorage)

      apiWorker = fluence.worker.Worker(params.appId, stateMachine, producer)

      responder ← WorkerResponder.make(apiWorker)

      services = new DockerWorkerServices[F](
        p2pPort,
        params.appId,
        producer,
        stateMachine,
        stateMachine.command[PeersControl[F]],
        responder,
        apiWorker.status(_)
      )

      // Start uploading tendermint blocks and send receipts to statemachine
      _ <- blockUploading.start(
        params.appId,
        blockManifests.receiptStorage,
        height ⇒ producer.blockStream(Some(height)),
        stateMachine.command[ReceiptBus[F]],
        blockManifests.onUploaded
      )

    } yield services

}
