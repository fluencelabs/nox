package fluence.node

import cats.data.NonEmptyList
import cats.effect.{ConcurrentEffect, Resource, Timer}
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.bp.embedded.EmbeddedBlockProducer
import fluence.effects.{Backoff, EffectError}
import fluence.log.Log
import fluence.node.workers.WorkersPorts
import fluence.statemachine.EmbeddedStateMachine
import fluence.statemachine.abci.peers.PeersControlBackend
import fluence.statemachine.api.command.PeersControl
import fluence.worker.eth.EthApp
import fluence.worker.{Worker, WorkerContext, WorkersPool}
import shapeless.{::, HNil}

import scala.language.higherKinds

object EmbeddedWorkerPool {

  private val moduleDirPrefix = if (System.getProperty("user.dir").endsWith("/statemachine")) "../" else "./"
  private val moduleFiles = List("counter.wast").map(moduleDirPrefix + "vm/src/test/resources/wast/" + _)

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
          machine <- EmbeddedStateMachine
            .init(NonEmptyList.fromList(moduleFiles).get, true)
            .value
            .map(_.right.get.extend(backend))
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
