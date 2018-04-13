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

package fluence.dataset.grpc.server

import cats.data.EitherT
import cats.effect.Async
import cats.{~>, Monad, Show}
import fluence.dataset.DatasetInfo
import monix.eval.Task
import monix.execution.Scheduler
import monix.reactive.{Observable, Observer}
import scodec.bits.{Bases, ByteVector}

import scala.util.{Failure, Success}

object DatasetServerOperation extends slogging.LazyLogging {

  def toF[F[_]: Async, E <: Throwable, V](
    eitherT: EitherT[Task, E, V]
  )(implicit runF: F ~> Task, scheduler: Scheduler): F[V] = {
    val fn = new (Task ~> F) { override def apply[A](fa: Task[A]): F[A] = fa.toIO.to[F] }
    eitherTaskToF[F, E, V](eitherT)(fn)
  }

  def toObservable[E <: Throwable, V](eitherT: EitherT[Task, E, V]): Observable[V] = {
    val fn = new (Task ~> Observable) { override def apply[A](fa: Task[A]): Observable[A] = Observable.fromTask(fa) }
    eitherTaskToF[Observable, E, V](eitherT)(fn)
  }

  implicit class ObserverCompleteWithOps[T](observer: Observer[T]) {

    def completeWith(task: Task[T])(implicit scheduler: Scheduler): Unit =
      task.runAsync.flatMap(observer.onNext).onComplete {
        case Success(_) ⇒ observer.onComplete()
        case Failure(ex) ⇒ observer.onError(ex)
      }
  }

  /**
   * Convert function: {{{ EitherT[Task, E, V] => Eff[V] }}}.
   * It's temporary decision, it will be removed when EitherT[F, E, V] was everywhere.
   *
   * @tparam E Type of exception, should be subclass of [[Throwable]]
   * @tparam V Type of value
   */
  def eitherTaskToF[Eff[_], E <: Throwable, V](eitherT: EitherT[Task, E, V])(implicit fn: Task ~> Eff): Eff[V] =
    fn(eitherT.value.flatMap {
      case Left(clientError) ⇒
        logger.debug(s"DatasetStorageServer lifts up client exception=$clientError")
        Task.raiseError(clientError)
      case Right(value) ⇒
        Task(value)
    })

  private val alphabet = Bases.Alphabets.Base64Url

  implicit val showBytes: Show[Array[Byte]] = (b: Array[Byte]) ⇒ ByteVector(b).toBase64(alphabet)

  implicit def showOption[T](implicit showT: Show[T]): Show[Option[T]] = { (o: Option[T]) ⇒
    o.map(showT.show).toString
  }

  implicit def showTuple[T](implicit showT: Show[T]): Show[(T, T)] = { (o: (T, T)) ⇒
    s"(${showT.show(o._1)},${showT.show(o._2)})"
  }

  implicit def showDatasetInfo(implicit showBytes: Show[Array[Byte]]): Show[DatasetInfo] = { (dsi: (DatasetInfo)) ⇒
    s"(DatasetInfo(id=${showBytes.show(dsi.id.toByteArray)},ver=${dsi.version})"
  }
}
