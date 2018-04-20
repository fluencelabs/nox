package fluence.transport.websocket

import io.grpc.stub.StreamObserver
import monix.eval.Task
import monix.execution.Ack.Continue
import monix.execution.{Ack, Scheduler}
import monix.reactive._
import org.scalajs.dom.{MessageEvent, WebSocket}

import scala.scalajs.js

object Main extends App {

  val overflow: OverflowStrategy.Synchronous[Nothing] = OverflowStrategy.Unbounded

  val webSocket = new WebSocket("")

  type I = String
  type O = String

  def pipeIn(ws: WebSocket) = {
    new Pipe[I, O] {
      override def unicast: (Observer[I], Observable[O]) = {

        val (in, out) = Observable.multicast[I](MulticastStrategy.replay, overflow)

        ws.onmessage = (event: MessageEvent) ⇒ {
          in.onNext(event.data.asInstanceOf[String])
        }

        //in is for pushing messages from websocket
        //out is observable for processing messages from websocket
        //  websocket.onmessage ---> in ---> out
        in -> out
      }
    }
  }

  def pipeOut(ws: WebSocket) =
    new Pipe[I, O] {
      override def unicast: (Observer[I], Observable[O]) = {

        val (in, out) = Observable.multicast[I](MulticastStrategy.replay, overflow)
        //in is for pushing messages from client
        //out is observable for senging messages to websocket
        //  in ---> out ---> websocket.send
        in -> Observable.create[O](overflow) { sync ⇒
          out.subscribe(new Observer.Sync[I] {
            def onNext(elem: I): Ack = {
              ws.send(elem)
              Continue
            }
            def onError(ex: Throwable): Unit = ()
            def onComplete(): Unit = ()
          })
        }
      }
    }

}
