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

package fluence.transport.websocket

import monix.execution.Ack.Continue
import monix.execution.{Ack, Cancelable}
import monix.reactive.Observable
import monix.reactive.observers.Subscriber
import monix.reactive.subjects.{PublishSubject, Subject}

import scala.collection.mutable
import scala.concurrent.Future
import monix.execution.Scheduler.Implicits.global

/**
 * Subject that cache input messages if there is no subscribers.
 * @tparam T In/out type.
 */
class SubjectQueue[T] extends Subject[T, T] {
  val publishSubject: PublishSubject[T] = PublishSubject[T]()
  val inputCache: mutable.Queue[T] = mutable.Queue.empty[T]

  override def unsafeSubscribeFn(subscriber: Subscriber[T]): Cancelable = {
    Observable
      .fromIterable(inputCache)
      .doOnNext(el ⇒ subscriber.onNext(el))
      .doOnComplete(() ⇒ inputCache.clear())
      .subscribe()

    publishSubject.subscribe(subscriber)
  }

  override def size: Int = publishSubject.size

  override def onNext(elem: T): Future[Ack] = {
    if (size > 0) publishSubject.onNext(elem)
    else {
      inputCache.enqueue(elem)
      Future(Continue)
    }
  }

  override def onError(ex: Throwable): Unit = publishSubject.onError(ex)

  override def onComplete(): Unit = publishSubject.onComplete()
}
