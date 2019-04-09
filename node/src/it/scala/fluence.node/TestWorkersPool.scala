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
import cats.effect.concurrent.MVar
import cats.effect.{Concurrent, Resource}
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.syntax.applicative._
import fluence.effects.docker.DockerContainerStopped
import fluence.effects.tendermint.rpc.TendermintRpc
import fluence.node.workers.control.ControlRpc
import fluence.node.workers.status.{HttpCheckNotPerformed, ServiceStatus, WorkerStatus}
import fluence.node.workers.{Worker, WorkerParams, WorkerServices, WorkersPool}

import scala.concurrent.duration.FiniteDuration
import scala.language.higherKinds

class TestWorkersPool[F[_]: Concurrent](workers: MVar[F, Map[Long, Worker[F]]]) extends WorkersPool[F] {

  /**
   * Run or restart a worker
   *
   * @param params Worker's description
   * @return Whether worker run or not
   */
  override def run(appId: Long, params: F[WorkerParams]): F[WorkersPool.RunResult] =
    workers.take.flatMap {
      case m if m.contains(appId) ⇒ workers.put(m).as(WorkersPool.AlreadyRunning)
      case m ⇒
        for {
          p ← params
          w ← Worker.make[F](
            appId,
            0: Short,
            s"Test worker for appId $appId",
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
            },
            identity,
            for {
              ws ← workers.take
              _ ← workers.put(ws - appId)
            } yield (),
            Applicative[F].unit
          ).allocated.map(_._1)
          _ ← workers.put(m + (appId -> w))
        } yield WorkersPool.Starting
    }

  /**
   * Get a Worker by its appId, if it's present
   *
   * @param appId Application id
   * @return Worker
   */
  override def get(appId: Long): F[Option[Worker[F]]] =
    workers.read.map(_.get(appId))

  /**
   * Get all known workers
   *
   * @return Up-to-date list of workers
   */
  override def getAll: F[List[Worker[F]]] =
    workers.read.map(_.values.toList)
}

object TestWorkersPool {

  def apply[F[_]: Concurrent]: F[TestWorkersPool[F]] =
    MVar.of(Map.empty[Long, Worker[F]]).map(new TestWorkersPool(_))

  def make[F[_]: Concurrent]: Resource[F, TestWorkersPool[F]] =
    Resource.liftF(apply[F])
}
