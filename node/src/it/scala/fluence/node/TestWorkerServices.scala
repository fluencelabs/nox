package fluence.node

import cats.Monad
import cats.effect.concurrent.Ref
import fluence.bp.api.BlockProducer
import fluence.effects.receipt.storage.ReceiptStorage
import fluence.effects.tendermint.block.history.BlockManifest
import fluence.node.workers.WorkerServices
import fluence.statemachine.api.StateMachine
import fluence.statemachine.api.command.PeersControl
import fluence.worker.WorkerStatus
import fluence.worker.responder.WorkerResponder

import scala.concurrent.duration.FiniteDuration
import scala.language.higherKinds

object TestWorkerServices {

  def emptyWorkerService[F[_]: Monad](bref: Ref[F, Option[BlockManifest]], bstore: ReceiptStorage[F])(
    appId: Long
  ): WorkerServices[F] = new WorkerServices[F] {
    override def producer: BlockProducer[F] = throw new NotImplementedError("def producer")
    override def machine: StateMachine[F] = throw new NotImplementedError("def machine")
    override def peersControl: PeersControl[F] = throw new NotImplementedError("def peersControl")
    override def status(timeout: FiniteDuration): F[WorkerStatus] = throw new NotImplementedError("def status")
    override def responder: WorkerResponder[F] = throw new NotImplementedError("def responder")
  }
}
