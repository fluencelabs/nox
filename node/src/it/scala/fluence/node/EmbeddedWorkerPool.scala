package fluence.node

import cats.data.NonEmptyList
import cats.effect.{ConcurrentEffect, Resource, Timer}
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.bp.embedded.EmbeddedBlockProducer
import fluence.bp.tx.TxResponse
import fluence.effects.{Backoff, EffectError}
import fluence.log.Log
import fluence.node.workers.WorkersPorts
import fluence.statemachine.EmbeddedStateMachine
import fluence.statemachine.abci.peers.PeersControlBackend
import fluence.statemachine.api.command.PeersControl
import fluence.statemachine.api.data.{StateHash, StateMachineStatus}
import fluence.statemachine.api.query.QueryResponse
import fluence.statemachine.receiptbus.ReceiptBusBackend
import fluence.statemachine.state.StateService
import fluence.worker.eth.EthApp
import fluence.worker.{Worker, WorkerContext, WorkersPool}
import shapeless.{::, HNil}
import sun.reflect.generics.reflectiveObjects.NotImplementedException

import scala.language.higherKinds

class TestStateService[F[_]] extends StateService[F] {
  override def stateHash: F[StateHash] = throw new NotImplementedError("def stateHash")
  override def commit(implicit log: Log[F]): F[StateHash] = throw new NotImplementedError("def commit")
  override def query(path: String): F[QueryResponse] = throw new NotImplementedError("def path")
  override def deliverTx(data: Array[Byte])(implicit log: Log[F]): F[TxResponse] =
    throw new NotImplementedError("def deliverTx")
  override def checkTx(data: Array[Byte])(implicit log: Log[F]): F[TxResponse] =
    throw new NotImplementedError("def checkTx")
}

object EmbeddedWorkerPool {

  type Resources[F[_]] = WorkersPorts.P2pPort[F] :: HNil
  type Companions[F[_]] = PeersControl[F] :: HNil

  def embedded[F[_]: ConcurrentEffect: Timer](ports: WorkersPorts[F])(
    implicit backoff: Backoff[EffectError],
    log: Log[F]
  ): F[WorkersPool[F, Resources[F], Companions[F]]] = {

    def embeddedWorker(app: EthApp): Resources[F] ⇒ Resource[F, Worker[F, Companions[F]]] = { res ⇒
      Resource.liftF(
        for {
          backend <- PeersControlBackend[F].map(a => a: PeersControl[F])
          receiptBus ← ReceiptBusBackend[F](false)
          machine = EmbeddedStateMachine(receiptBus,
                                         new TestStateService[F](),
                                         StateMachineStatus(false, StateHash.empty)).extend(backend)
          producer <- EmbeddedBlockProducer(machine)
        } yield
          Worker(
            app.id,
            machine,
            producer,
            machine.command[PeersControl[F]] :: HNil
          )
      )
    }

    WorkersPool[F, Resources[F], Companions[F]] { (app, l) ⇒
      implicit val log: Log[F] = l
      WorkerContext(
        app,
        ports.workerResource(app.id).map(_ :: HNil),
        embeddedWorker(app)
      )
    }
  }
}
