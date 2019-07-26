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

import scala.concurrent.duration._
import scala.concurrent.duration.FiniteDuration
import scala.language.higherKinds

object TestWorkerServices {

  def emptyWorkerService[F[_]: Monad](bref: Ref[F, Option[BlockManifest]],
                                      bstore: ReceiptStorage[F])(appId: Long): WorkerServices[F] = {
    new WorkerServices[F] {
      override def tendermint: TendermintRpc[F] = ???

      override def control: ControlRpc[F] = ???

      override def status(timeout: FiniteDuration): F[WorkerStatus] =
        WorkerStatus(
          isHealthy = true,
          appId = appId,
          ServiceStatus(Left(DockerContainerStopped(0)), HttpCheckNotPerformed("dumb")),
          ServiceStatus(Left(DockerContainerStopped(0)), HttpCheckNotPerformed("dumb"))
        ).pure[F]

      override def blockManifests: WorkerBlockManifests[F] = new WorkerBlockManifests(bstore, bref)
    }
  }

  def workerServiceTestRequestResponse[F[_]: Applicative: Timer](appId: Long): WorkerServices[F] = {
    new WorkerServices[F] {
      override def tendermint: TendermintRpc[F] = new TestTendermintRpc[F] with TestTendermintWebsocketRpc[F] {
        override def subscribeNewBlock(lastKnownHeight: Long)(implicit log: Log[F],
                                                              backoff: Backoff[EffectError]): fs2.Stream[F, Block] =
          fs2.Stream
            .awakeEvery[F](500.milliseconds)
            .map(_ => Block(TestData.blockWithNullTxsResponse(1)).right.get)

        override def consensusHeight(id: String): EitherT[F, RpcError, Long] =
          EitherT.pure(0)

        override def broadcastTxSync(tx: String, id: String): EitherT[F, RpcError, String] =
          EitherT.pure("""
                         |{
                         |
                         |
                         |    "error": "",
                         |    "result": {
                         |        "hash": "2B8EC32BA2579B3B8606E42C06DE2F7AFA2556EF",
                         |        "log": "",
                         |        "data": "",
                         |        "code": "0"
                         |    },
                         |    "id": "",
                         |    "jsonrpc": "2.0"
                         |
                         |}
                         |""".stripMargin)

        override def query(
          path: String,
          data: String,
          height: Long,
          prove: Boolean,
          id: String
        ): EitherT[F, RpcError, String] = EitherT.pure("""
                                                         |{
                                                         |
                                                         |
                                                         |    "error": "",
                                                         |    "result": {
                                                         |        "response": {
                                                         |            "log": "exists",
                                                         |            "height": "0",
                                                         |            "value": "61626364",
                                                         |            "key": "61626364",
                                                         |            "index": "-1",
                                                         |            "code": "0"
                                                         |        }
                                                         |    },
                                                         |    "id": "",
                                                         |    "jsonrpc": "2.0"
                                                         |
                                                         |}
                                                         |""".stripMargin)
      }

      override def control: ControlRpc[F] = ???

      override def status(timeout: FiniteDuration): F[WorkerStatus] =
        WorkerStatus(
          isHealthy = true,
          appId = appId,
          ServiceStatus(Left(DockerContainerStopped(0)), HttpCheckNotPerformed("dumb")),
          ServiceStatus(Left(DockerContainerStopped(0)), HttpCheckNotPerformed("dumb"))
        ).pure[F]

      override def blockManifests: WorkerBlockManifests[F] = ???
    }
  }
}
