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

package fluence.worker

import cats.Functor
import cats.data.EitherT
import cats.syntax.flatMap._
import cats.syntax.apply._
import cats.syntax.applicative._
import cats.syntax.functor._
import cats.syntax.either._
import cats.effect.{Concurrent, Fiber, Resource}
import cats.effect.concurrent.{Deferred, Ref}
import fluence.effects.resources.MakeResource
import fluence.log.Log
import fluence.worker.eth.EthApp
import shapeless._

import scala.language.higherKinds

/**
 * WorkerContext describes all the resources used by Worker and their lifecycles.
 *
 * Control ---- Join! ------------------ Stop! ------------------------------ Destroy! ----------------------------
 * Context ---- Init ----------------------------------------(may be closed)----------------- Not usable anymore --
 * Resources ------ Prepare ---------------------------------(kept in place)-----X-- Destroy ----------------------
 * Worker ------------------- Allocate ----X---- Deallocate -------------------------------------------------------
 *
 * @param stage Fast accessor to current WorkerStage
 * @param stages Stream of WorkerStage changes, can be instantiated several times
 * @param app EthApp for which this context is built
 * @param resources Resources
 * @param worker Either current non-active WorkerStage, or fully initialized Worker
 */
abstract class WorkerContext[F[_]: Functor, R, CS <: HList](
  val stage: F[WorkerStage],
  val stages: fs2.Stream[F, WorkerStage],
  val app: EthApp,
  val resources: R,
  val worker: EitherT[F, WorkerStage, Worker[F, CS]]
) {

  /**
   * Pick a Worker Companion, if Worker is running, or report current non-active WorkerStage
   *
   * @tparam C Companion type
   * @return WorkerStage if worker is not launched, or Companion instance
   */
  def companion[C](implicit c: ops.hlist.Selector[CS, C]): EitherT[F, WorkerStage, C] =
    worker.map(_.companion[C])

  /**
   * Trigger Worker stop, releasing all acquired resources.
   * Stopping is performed asynchronously, you can track it with [[stage]]
   *
   */
  def stop()(implicit log: Log[F]): F[Unit]

  /**
   * Stop the worker and then destroy all the prepared [[resources]].
   * Worker must be re-initialized from scratch after that. Operation cannot be reverted.
   * Destruction is performed asynchronously, you can track it with [[stage]]
   */
  def destroy()(implicit log: Log[F]): F[Fiber[F, Unit]]
}

object WorkerContext {

  def apply[F[_]: Concurrent, R, CS <: HList](
    app: EthApp,
    workerResource: WorkerResource[F, R],
    worker: R ⇒ Resource[F, Worker[F, CS]]
  )(implicit log: Log[F]): F[WorkerContext[F, R, CS]] =
    workerResource.prepare() >>= { res ⇒
      for {
        // Provide WorkerStage info within Ref for regular access, and with Queue to enable subscriptions
        stageRef ← Ref.of[F, WorkerStage](WorkerStage.NotInitialized)
        stageQueue ← fs2.concurrent.Topic[F, WorkerStage](WorkerStage.NotInitialized)

        // Push stage updates to both queue and ref
        setStage = (s: WorkerStage) ⇒
          stageQueue.publish1(s) *>
            stageRef.set(s) *>
            log.info(s"Worker ${app.id} stage changed to $s")

        // We will get Worker, Companions and Stop callback later
        // TODO: if we want context to be restartable, these needs to be Queues?
        workerDef ← Deferred[F, Worker[F, CS]]
        stopDef ← Deferred[F, F[Unit]]

        // Allocate Worker and Resources with no blocking
        _ ← MakeResource.useConcurrently[F](
          stop ⇒
            // Wrap the whole Worker/Companions lifecycle with stage-reflecting resource
            Resource.make[F, Unit](
              setStage(WorkerStage.InitializationStarted) >> stopDef.complete(stop)
            )(_ ⇒ setStage(WorkerStage.Stopped)) >>
              // Worker allocate function may take a lot of time: it waits for Resources, runs Docker, etc.
              worker(res)
                .flatTap(
                  w ⇒
                    // Everything is ready
                    Resource.liftF(
                      workerDef.complete(w) *>
                        setStage(WorkerStage.FullyAllocated)
                    )
                )
        )

        stage = stageRef.get

      } yield new WorkerContext[F, R, CS](
        stage,
        stageQueue.subscribe(1),
        app,
        res,
        EitherT(stage.flatMap {
          case s if s.running ⇒ workerDef.get.map(_.asRight)
          case s ⇒ s.asLeft[Worker[F, CS]].pure[F]
        })
      ) {
        override def stop()(implicit log: Log[F]): F[Unit] =
          stage.flatMap {
            case s if s.running ⇒
              log.info(s"Stopping worker ${app.id}") >>
                setStage(WorkerStage.Stopping) >>
                stopDef.get.flatten
            case _ ⇒
              log.debug(s"Worker ${app.id} is already stopped")
          }

        // TODO: if it's already destroyed, we can't return a Fiber
        override def destroy()(implicit log: Log[F]): F[Fiber[F, Unit]] =
          log.info(s"Going to destroy worker ${app.id}") >>
            stop() >>
            Concurrent[F].start(
              setStage(WorkerStage.Destroying) >>
                workerResource.destroy().value.void >>
                setStage(WorkerStage.Destroyed)
            )
      }

    }
}
