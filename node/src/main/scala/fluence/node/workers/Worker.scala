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

import cats.effect.{Concurrent, Resource}
import cats.effect.concurrent.Deferred
import cats.syntax.apply._
import cats.syntax.flatMap._
import cats.syntax.functor._

import scala.concurrent.duration.FiniteDuration
import scala.language.higherKinds

/**
 * Worker is a representation of App Worker, incapsulating WorkerServices,
 * and ordered execution via [[withServices_]] and [[withServices]].
 *
 * @param appId AppId of the application hosted by this worker
 * @param p2pPort Tendermint p2p port
 * @param services WorkerServices (Tendermint, ControlRPC)
 * @param description Human readable description of the worker
 * @param execute Description of how to execute F[Unit] in Worker's context. As of now, preserves ordered of the execution.
 * @param stop Delayed effect, when executed, stops the worker and deallocates resources
 * @param remove Delayed effect, when executed, removes worker and removes some resources
 */
case class Worker[F[_]: Concurrent] private (
  appId: Long,
  p2pPort: Short,
  services: WorkerServices[F],
  description: String,
  private val execute: F[Unit] ⇒ F[Unit],
  stop: F[Unit],
  remove: F[Unit]
) {

  // Reports this worker's health
  def isHealthy(timeout: FiniteDuration): F[Boolean] =
    services.status(timeout).map(_.isHealthy)

  // Executes fn * f in worker's context, keeping execution order. Discards the result.
  def withServices_[T, A](f: WorkerServices[F] ⇒ T)(fn: T ⇒ F[A]): F[Unit] =
    execute(
      fn(f(services)).void
    )

  // Executes fn * f in worker's context, keeping execution order. Returns the result.
  def withServices[T, A](f: WorkerServices[F] ⇒ T)(fn: T ⇒ F[A]): F[A] =
    for {
      d ← Deferred[F, A]
      _ ← withServices_(f)(t ⇒ fn(t).flatMap(d.complete))
      r ← d.get
    } yield r
}

object Worker {

  /**
   * Builds a Worker, executing workerRun as a first worker's command
   *
   * @param appId AppId of the application hosted by this worker
   * @param p2pPort Tendermint p2p port
   * @param description Human readable description of the worker
   * @param services Worker services
   * @param onStop Callback, called on worker's stop, but only after all commands have been processed
   * @param onRemove Callback, called on worker's removal, but only after all commands have been processed
   * @return A Worker's instance, that will initialize itself in the background
   */
  def make[F[_]: Concurrent](
    appId: Long,
    p2pPort: Short,
    description: String,
    services: WorkerServices[F],
    scheduleExecution: F[Unit] ⇒ F[Unit],
    onStop: F[Unit],
    onRemove: F[Unit]
  ): Resource[F, Worker[F]] =
    Resource.pure(
      Worker[F](
        appId,
        p2pPort,
        services,
        description,
        scheduleExecution,
        onStop,
        onStop *> onRemove
      )
    )

}
