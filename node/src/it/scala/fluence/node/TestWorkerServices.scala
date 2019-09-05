package fluence.node

import cats.effect.concurrent.Ref
import cats.effect.Timer
import cats.{Applicative, Monad}
import fluence.effects.docker.DockerContainerStopped
import fluence.node.workers.{WorkerBlockManifests, WorkerServices}
import fluence.node.workers.control.ControlRpc
import fluence.node.workers.status.{HttpCheckNotPerformed, ServiceStatus, WorkerStatus}
import cats.syntax.applicative._
import fluence.effects.receipt.storage.ReceiptStorage
import fluence.effects.tendermint.block.history.BlockManifest
import fluence.effects.tendermint.rpc.http.TendermintHttpRpc
import fluence.effects.tendermint.rpc.websocket.TendermintWebsocketRpc
import fluence.node.workers.subscription.{ResponseSubscriber, StoredProcedureExecutor, WaitResponseService}

import scala.concurrent.duration.FiniteDuration
import scala.language.higherKinds

object TestWorkerServices {

  def emptyWorkerService[F[_]: Monad](bref: Ref[F, Option[BlockManifest]], bstore: ReceiptStorage[F])(
    appId: Long
  ): WorkerServices[F] = {
    new WorkerServices[F] {
      override def tendermintRpc: TendermintHttpRpc[F] = throw new NotImplementedError("def tendermintRpc")
      override def tendermintWRpc: TendermintWebsocketRpc[F] = throw new NotImplementedError("def tendermintWRpc")

      override def control: ControlRpc[F] = throw new NotImplementedError("def control")

      override def status(timeout: FiniteDuration): F[WorkerStatus] =
        WorkerStatus(
          isHealthy = true,
          appId = appId,
          ServiceStatus(Left(DockerContainerStopped(0)), HttpCheckNotPerformed("dumb")),
          ServiceStatus(Left(DockerContainerStopped(0)), HttpCheckNotPerformed("dumb"))
        ).pure[F]

      override def blockManifests: WorkerBlockManifests[F] = new WorkerBlockManifests(bstore, bref)

      override def waitResponseService: WaitResponseService[F] = throw new NotImplementedError("def requestResponder")

      override def storedProcedureExecutor: StoredProcedureExecutor[F] = ???
    }
  }

  def workerServiceTestRequestResponse[F[_]: Monad: Timer](
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

      override def waitResponseService: WaitResponseService[F] = WaitResponseService(rpc, requestResponderImpl)

      override def storedProcedureExecutor: StoredProcedureExecutor[F] = ???
    }
  }
}
