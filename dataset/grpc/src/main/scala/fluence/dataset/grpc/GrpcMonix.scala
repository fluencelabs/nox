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

package fluence.dataset.grpc

import io.grpc.stub.StreamObserver
import monix.eval.{ MVar, Task }
import monix.execution.Scheduler.Implicits.global
import monix.execution.{ Ack, Cancelable }
import monix.reactive._

import scala.concurrent.Future
import scala.language.implicitConversions
import scala.util.{ Failure, Success }

object GrpcMonix {

  private val overflow: OverflowStrategy.Synchronous[Nothing] = OverflowStrategy.Unbounded

  def callToPipe[I, O](call: StreamObserver[O] ⇒ StreamObserver[I]): Pipe[I, O] = new Pipe[I, O] {
    override def unicast: (Observer[I], Observable[O]) = {

      val (in, inOut) = Observable.multicast[I](MulticastStrategy.replay, overflow)

      in -> Observable.create[O](overflow) { sync ⇒
        inOut.subscribe(streamToObserver(call(new StreamObserver[O] {
          override def onError(t: Throwable): Unit =
            sync.onError(t)

          override def onCompleted(): Unit =
            sync.onComplete()

          override def onNext(value: O): Unit =
            sync.onNext(value).failed.foreach(onError)
        })))
      }
    }
  }

  implicit def streamToObserver[T](stream: StreamObserver[T]): Observer[T] = new Observer[T] {
    override def onError(ex: Throwable): Unit =
      stream.onError(ex)

    override def onComplete(): Unit =
      stream.onCompleted()

    override def onNext(elem: T): Future[Ack] =
      Task(stream.onNext(elem))
        .map(_ ⇒ Ack.Continue)
        .onErrorHandle {
          t ⇒
            onError(t)
            Ack.Stop
        }.runAsync
  }

  def streamObservable[T]: (Observable[T], StreamObserver[T]) = {
    val (in, out) = Observable.multicast[T](MulticastStrategy.replay, overflow)

    (out, new StreamObserver[T] {
      override def onError(t: Throwable): Unit =
        in.onError(t)

      override def onCompleted(): Unit =
        in.onComplete()

      override def onNext(value: T): Unit =
        in.onNext(value)
    })
  }

  implicit class ObservableGrpcOps[T](observable: Observable[T]) {
    def pipeTo(stream: StreamObserver[T]): Cancelable =
      observable.subscribe(stream: Observer[T])

    def pullable: () ⇒ Task[T] = {
      val variable = MVar.empty[T]

      observable.subscribe(
        nextFn = v ⇒ variable.put(v).map(_ ⇒ Ack.Continue).runAsync
      )

      () ⇒ variable.take
    }
  }

  implicit class ObserverGrpcOps[T](observer: Observer[T]) {
    def completeWith(task: Task[T]): Unit =
      task.runAsync.flatMap(observer.onNext).onComplete {
        case Success(_)  ⇒ observer.onComplete()
        case Failure(ex) ⇒ observer.onError(ex)
      }
  }
}
