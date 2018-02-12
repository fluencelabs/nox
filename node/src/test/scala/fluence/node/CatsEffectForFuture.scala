/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

package fluence.node

import cats.effect.{ Effect, IO }
import monix.eval.Task
import monix.eval.instances.CatsEffectForTask
import monix.execution.{ CancelableFuture, Scheduler }

import scala.concurrent.Future

/**
 * Implementation [[cats.effect.Effect]] for [[scala.concurrent.Future]].
 */
class CatsEffectForFuture(implicit ec: Scheduler) extends Effect[Future] {

  private val taskEff = new CatsEffectForTask()

  override def runAsync[A](fa: Future[A])(cb: (Either[Throwable, A]) ⇒ IO[Unit]): IO[Unit] =
    taskEff.runAsync(Task.fromFuture(fa))(cb)

  override def async[A](k: ((Either[Throwable, A]) ⇒ Unit) ⇒ Unit): Future[A] =
    taskEff.async(k).runAsync

  override def suspend[A](thunk: ⇒ Future[A]): Future[A] =
    taskEff.suspend(Task.fromFuture(thunk)).runAsync

  override def flatMap[A, B](fa: Future[A])(f: (A) ⇒ Future[B]): Future[B] =
    fa.flatMap(f)

  override def tailRecM[A, B](a: A)(f: (A) ⇒ Future[Either[A, B]]): CancelableFuture[B] =
    taskEff.tailRecM(a) { it ⇒ Task.fromFuture(f(it)) }.runAsync

  override def raiseError[A](e: Throwable): Future[A] =
    Future.failed(e)

  override def handleErrorWith[A](fa: Future[A])(f: (Throwable) ⇒ Future[A]): Future[A] =
    fa.recoverWith { case t ⇒ f(t) }

  override def pure[A](x: A): Future[A] =
    Future(x)

}
