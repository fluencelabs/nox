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

package fluence.grpc

import cats.effect.IO
import com.google.common.util.concurrent.ListenableFuture
import io.grpc.stub.StreamObserver
import monix.execution.Ack.{Continue, Stop}
import monix.execution.{Ack, Cancelable, Scheduler}
import monix.reactive.Observable.Operator
import monix.reactive.Observer.Sync
import monix.reactive._
import monix.reactive.observers.Subscriber
import scalapb.grpc.Grpc

import scala.concurrent.Future
import scala.language.implicitConversions

object GrpcMonix {

  type GrpcOperator[I, O] = StreamObserver[O] ⇒ StreamObserver[I]

  def liftByGrpcOperator[I, O](observable: Observable[I], operator: GrpcOperator[I, O]): Observable[O] =
    observable.liftByOperator(
      grpcOperatorToMonixOperator(operator)
    )

  def grpcOperatorToMonixOperator[I, O](grpcOperator: GrpcOperator[I, O]): Operator[I, O] = {
    outputSubsriber: Subscriber[O] ⇒
      val outputObserver: StreamObserver[O] = monixSubscriberToGrpcObserver(outputSubsriber)
      val inputObserver: StreamObserver[I] = grpcOperator(outputObserver)
      grpcObserverToMonixSubscriber(inputObserver, outputSubsriber.scheduler)
  }

  def monixSubscriberToGrpcObserver[T](subscriber: Subscriber[T]): StreamObserver[T] =
    new StreamObserver[T] {
      override def onError(t: Throwable): Unit = subscriber.onError(t)
      override def onCompleted(): Unit = subscriber.onComplete()
      override def onNext(value: T): Unit = subscriber.onNext(value)
    }

  def grpcObserverToMonixSubscriber[T](observer: StreamObserver[T], s: Scheduler): Subscriber[T] =
    new Subscriber[T] {
      override implicit def scheduler: Scheduler = s
      override def onError(t: Throwable): Unit = observer.onError(t)
      override def onComplete(): Unit = observer.onCompleted()
      override def onNext(value: T): Future[Ack] =
        try {
          observer.onNext(value)
          Continue
        } catch {
          case t: Throwable ⇒
            observer.onError(t)
            Stop
        }
    }

  private val overflow: OverflowStrategy.Synchronous[Nothing] = OverflowStrategy.Unbounded

  implicit def streamToObserver[T](stream: StreamObserver[T])(implicit sch: Scheduler): Observer.Sync[T] =
    new Observer.Sync[T] {
      override def onError(ex: Throwable): Unit =
        stream.onError(ex)

      override def onComplete(): Unit =
        stream.onCompleted()

      override def onNext(elem: T): Ack = {
        stream.onNext(elem)
        Ack.Continue
      }
    }

  def guavaFutureToIO[T](future: ListenableFuture[T]): IO[T] =
    IO.fromFuture(IO(Grpc.guavaFuture2ScalaFuture(future)))

  implicit def observerToStream[T](observer: Sync[T])(implicit sch: Scheduler): StreamObserver[T] = {
    new StreamObserver[T] {
      override def onNext(value: T): Unit = observer.onNext(value)

      override def onError(t: Throwable): Unit = observer.onError(t)

      override def onCompleted(): Unit = observer.onComplete()
    }
  }

  def streamObservable[T](implicit sch: Scheduler): (Observable[T], StreamObserver[T]) = {
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

    def pipeTo(stream: StreamObserver[T])(implicit sch: Scheduler): Cancelable =
      observable.subscribe(stream: Observer[T])
  }
}
