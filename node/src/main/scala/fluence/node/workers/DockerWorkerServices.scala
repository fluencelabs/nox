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
import cats.syntax.apply._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.{Monad, Parallel}
import fluence.bp.tendermint.Tendermint
import fluence.effects.docker._
import fluence.effects.receipt.storage.ReceiptStorage
import fluence.effects.sttp.SttpEffect
import fluence.effects.tendermint.rpc.http.TendermintHttpRpc
import fluence.effects.tendermint.rpc.websocket.WebsocketConfig
import fluence.effects.{Backoff, EffectError}
import fluence.log.Log
import fluence.node.status.StatusHttp
import fluence.node.workers.pool.WorkerP2pConnectivity
import fluence.node.workers.status._
import fluence.node.workers.subscription.{PerBlockTxExecutor, ResponseSubscriber, WaitResponseService}
import fluence.node.workers.tendermint.DockerTendermint
import fluence.node.workers.tendermint.block.BlockUploading
import fluence.statemachine.api.StateMachine
import fluence.statemachine.api.command.{PeersControl, ReceiptBus}
import fluence.statemachine.api.data.StateMachineStatus
import fluence.statemachine.docker.DockerStateMachine

import scala.concurrent.duration.FiniteDuration
import scala.language.higherKinds

/**
 * Single running worker's datatype
 *
 * @param p2pPort Tendermint p2p port
 * @param appId Worker's app ID
 * @param tendermint Tendermint HTTP RPC endpoints for the worker TODO replace with block producer
 * @param receiptBus Control RPC endpoints for the worker
 * @param statusCall Getter for actual Worker's status
 * @tparam F the effect
 */
case class DockerWorkerServices[F[_]] private (
  p2pPort: Short,
  appId: Long,
  tendermint: Tendermint[F],
  machine: StateMachine[F],
  receiptBus: ReceiptBus[F],
  peersControl: PeersControl[F],
  blockManifests: WorkerBlockManifests[F],
  waitResponseService: WaitResponseService[F],
  perBlockTxExecutor: PerBlockTxExecutor[F],
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
   * Wait until Tendermint container is started, and return a description of how to retrieve worker status
   */
  // TODO: wait until RPC is avalable. BLOCKED: RPC isn't available during block replay =>
  //  need to separate block uploading into replay and usual block processing parts
  private def appStatus[F[_]: DockerIO: Timer: ConcurrentEffect: ContextShift](
    stateMachine: StateMachine[F],
    stateMachineContainer: DockerContainer,
    tendermint: DockerTendermint,
    rpc: TendermintHttpRpc[F],
    appId: Long,
    timeout: FiniteDuration = StatusHttp.DefaultTimeout
  )(implicit log: Log[F]): Resource[F, FiniteDuration => F[WorkerStatus]] = Resource.liftF {
    def workerStatus(tout: FiniteDuration) =
      DockerIO[F]
        .checkContainer(stateMachineContainer)
        .semiflatMap[ServiceStatus[StateMachineStatus]] { d ⇒
          HttpStatus
            .timed(stateMachine.status.value.map[HttpStatus[StateMachineStatus]] {
              case Right(st) ⇒ HttpCheckStatus(st)
              case Left(err) ⇒ HttpCheckFailed(err)
            }, tout)
            .map(s ⇒ ServiceStatus(Right(d), s))
        }
        .valueOr(err ⇒ ServiceStatus(Left(err), HttpCheckNotPerformed("Worker's Docker container is not launched")))

    def tendermintStatus(tout: FiniteDuration) = tendermint.status(rpc, tout)

    def status(tout: FiniteDuration): F[WorkerStatus] = (tendermintStatus(tout), workerStatus(tout)).mapN { (ts, ws) ⇒
      WorkerStatus(
        isHealthy = ts.isOk(_.sync_info.latest_block_height > 0) && ws.isOk(),
        appId,
        ts,
        ws
      )
    }

    Log[F].scope("appStatus") { implicit log: Log[F] =>
      Log[F].info(s"Waiting for tendermint container to start") >>
        Monad[F]
          .tailRecM(tendermintStatus(timeout))(
            _.flatMap(
              s =>
                Log[F]
                  .debug(s"$s")
                  .as(Either.cond(s.isDockerRunning, status(_), Timer[F].sleep(timeout) >> tendermintStatus(timeout)))
            )
          )
          .flatTap(_ => Log[F].info("Tendermint has started"))
    }
  }

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

      tendermint ← DockerTendermint.make[F](params, p2pPort, containerName(params), network, stopTimeout)

      tm ← Tendermint.make[F](tendermint.name, DockerTendermint.RpcPort, params.tendermintPath, websocketConfig)

      // Once tendermint is started, run background job to connect it to all the peers
      _ ← WorkerP2pConnectivity.make(params.app.id, tm.rpc, params.app.cluster.workers)

      // Wait until tendermint container is started
      // TODO: wait until RPC is avalable. BLOCKED: RPC isn't available during block replay =>
      //  need to separate block uploading into replay and usual block processing parts
      status <- appStatus(stateMachine, stateMachine.command[DockerContainer], tendermint, tm.rpc, params.appId)

      blockManifests ← WorkerBlockManifests.make[F](receiptStorage)

      responseSubscriber <- ResponseSubscriber.make(tm.rpc, tm.wrpc, params.appId)

      waitResponseService ← WaitResponseService(tm.rpc, responseSubscriber)

      storedProcedureExecutor <- PerBlockTxExecutor
        .make(tm.wrpc, tm.rpc, waitResponseService)

      services = new DockerWorkerServices[F](
        p2pPort,
        params.appId,
        tm,
        stateMachine,
        stateMachine.command[ReceiptBus[F]],
        stateMachine.command[PeersControl[F]],
        blockManifests,
        waitResponseService,
        storedProcedureExecutor,
        status
      )

      // Start uploading tendermint blocks and send receipts to statemachine
      _ <- blockUploading.start(params.app.id, services)

    } yield services

}
