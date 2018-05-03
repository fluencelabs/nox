package fluence.transport.websocket

import monix.execution.Ack.Continue
import monix.execution.{Ack, Cancelable}
import monix.reactive.Observable
import monix.reactive.observers.Subscriber
import monix.reactive.subjects.{PublishSubject, Subject}

import scala.collection.mutable
import scala.concurrent.Future
import monix.execution.Scheduler.Implicits.global

class QueueingSubject[T] extends Subject[T, T] {
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
