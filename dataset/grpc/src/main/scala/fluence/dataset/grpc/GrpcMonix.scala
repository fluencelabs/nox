package fluence.dataset.grpc

import io.grpc.stub.StreamObserver
import monix.eval.Task
import monix.execution.Ack
import monix.execution.Scheduler.Implicits.global
import monix.reactive._

import scala.concurrent.Future
import scala.language.implicitConversions

object GrpcMonix {

  def callToPipe[I, O](call: StreamObserver[O] ⇒ StreamObserver[I]): Pipe[I, O] = new Pipe[I, O] {
    override def unicast: (Observer[I], Observable[O]) = {

      val (in, inOut) = Observable.multicast[I](MulticastStrategy.replay, OverflowStrategy.Fail(1))

      in -> Observable.create[O](OverflowStrategy.Fail(1)) { sync ⇒
        inOut.subscribe(streamToMonix(call(new StreamObserver[O] {
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

  implicit def streamToMonix[T](stream: StreamObserver[T]): Observer[T] = new Observer[T] {
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
}
