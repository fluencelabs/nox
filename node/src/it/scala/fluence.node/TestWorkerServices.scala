package fluence.node

import cats.data.EitherT
import cats.effect.concurrent.Ref
import cats.effect.{Resource, Timer}
import cats.{Applicative, Functor, Monad}
import fluence.effects.docker.DockerContainerStopped
import fluence.effects.tendermint.rpc.TendermintRpc
import fluence.node.workers.{WorkerBlockManifests, WorkerServices}
import fluence.node.workers.control.ControlRpc
import fluence.node.workers.status.{HttpCheckNotPerformed, ServiceStatus, WorkerStatus}
import cats.syntax.applicative._
import fluence.effects.receipt.storage.ReceiptStorage
import fluence.effects.tendermint.block.TestData
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.block.history.BlockManifest
import fluence.effects.{Backoff, EffectError}
import fluence.effects.tendermint.rpc.http.RpcError
import fluence.effects.tendermint.rpc.websocket.{Event, TestTendermintRpc, TestTendermintWebsocketRpc}
import fluence.log.Log
import fluence.node.workers.subscription.ResponseSubscriber

import scala.concurrent.duration._
import scala.concurrent.duration.FiniteDuration
import scala.language.higherKinds

object TestWorkerServices {

  def emptyWorkerService[F[_]: Monad](bref: Ref[F, Option[BlockManifest]],
                                      bstore: ReceiptStorage[F])(appId: Long): WorkerServices[F] = {
    new WorkerServices[F] {
      override def tendermint: TendermintRpc[F] = throw new NotImplementedError("def tendermint")

      override def control: ControlRpc[F] = throw new NotImplementedError("def control")

      override def status(timeout: FiniteDuration): F[WorkerStatus] =
        WorkerStatus(
          isHealthy = true,
          appId = appId,
          ServiceStatus(Left(DockerContainerStopped(0)), HttpCheckNotPerformed("dumb")),
          ServiceStatus(Left(DockerContainerStopped(0)), HttpCheckNotPerformed("dumb"))
        ).pure[F]

      override def blockManifests: WorkerBlockManifests[F] = new WorkerBlockManifests(bstore, bref)

      override def responseSubscriber: ResponseSubscriber[F] = throw new NotImplementedError("def requestResponder")
    }
  }

  def workerServiceTestRequestResponse[F[_]: Applicative: Timer](
    tendermintRpc: TendermintRpc[F],
    requestResponderImpl: ResponseSubscriber[F]
  )(appId: Long): WorkerServices[F] = {
    new WorkerServices[F] {
      override def tendermint: TendermintRpc[F] = tendermintRpc

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
