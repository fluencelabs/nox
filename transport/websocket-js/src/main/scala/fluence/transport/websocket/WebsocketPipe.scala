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

import fluence.proxy.grpc.WebsocketMessage
import monix.execution.{Ack, Scheduler}
import monix.reactive._

import scala.concurrent.Future
import scala.language.higherKinds

case class WebsocketPipe[A, B] private (
  input: Observer[A],
  output: Observable[B],
  statusOutput: Observable[StatusFrame]
) {

  def xmap[A1, B1](inputCodec: A1 ⇒ A, outputCodec: B ⇒ B1): WebsocketPipe[A1, B1] = {

    val tObserver: Observer[A1] = new Observer[A1] {
      override def onNext(elem: A1): Future[Ack] = {
        val message = inputCodec(elem)
        input.onNext(message)
      }

      override def onError(ex: Throwable): Unit = input.onError(ex)

      override def onComplete(): Unit = input.onComplete()
    }

    val tObservable = output.map(outputCodec)

    WebsocketPipe(tObserver, tObservable, statusOutput)
  }
}

object WebsocketPipe {

  type WebsocketClient[T] = WebsocketPipe[T, T]

  /**
   *
   * @param url Address to connect by websocket
   * @return An observer that will be an input into a websocket and observable - output
   */
  def apply(url: String, builder: String ⇒ WebsocketT)(
    implicit scheduler: Scheduler
  ): WebsocketClient[WebsocketFrame] = {

    val queueingSubject = new SubjectQueue[WebsocketFrame]

    val (statusInput, statusOutput) = Observable.multicast(MulticastStrategy.publish[StatusFrame])

    val observable = new WebsocketObservable(url, builder, queueingSubject, statusInput)

    val hotObservable = observable.multicast(Pipe.publish[WebsocketFrame])
    hotObservable.connect()

    WebsocketPipe(queueingSubject, hotObservable, statusOutput)
  }

  /**
   * Client that accepts only binary data.
   */
  def binaryClient(
    url: String,
    builder: String ⇒ WebsocketT
  )(implicit scheduler: Scheduler): WebsocketClient[Array[Byte]] = {

    val WebsocketPipe(wsObserver, wsObservable, statusOutput) = WebsocketPipe(url, builder)

    val binaryClient: Observer[Array[Byte]] = new Observer[Array[Byte]] {
      override def onNext(elem: Array[Byte]): Future[Ack] = wsObserver.onNext(Binary(elem))

      override def onError(ex: Throwable): Unit = wsObserver.onError(ex)

      override def onComplete(): Unit = wsObserver.onComplete()
    }

    val binaryObservable = wsObservable.collect {
      case Binary(data) ⇒ data
    }

    WebsocketPipe(binaryClient, binaryObservable, statusOutput)
  }
}
