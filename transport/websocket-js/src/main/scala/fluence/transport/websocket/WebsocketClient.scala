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

import monix.execution.{Ack, Scheduler}
import monix.reactive._
import scodec.bits.ByteVector

import scala.concurrent.Future
import scala.language.higherKinds

case class WebsocketClient[T] private (
  input: Observer[T],
  output: Observable[T],
  statusOutput: Observable[StatusFrame]
)

object WebsocketClient {

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

    WebsocketClient(queueingSubject, hotObservable, statusOutput)
  }

  /**
   * Client that accepts only binary data.
   */
  def binaryClient(
    url: String,
    builder: String ⇒ WebsocketT
  )(implicit scheduler: Scheduler): WebsocketClient[Array[Byte]] = {

    val WebsocketClient(wsObserver, wsObservable, statusOutput) = WebsocketClient(url, builder)

    val binaryClient: Observer[Array[Byte]] = new Observer[Array[Byte]] {
      override def onNext(elem: Array[Byte]): Future[Ack] = wsObserver.onNext(Binary(elem))

      override def onError(ex: Throwable): Unit = wsObserver.onError(ex)

      override def onComplete(): Unit = wsObserver.onComplete()
    }

    val binaryObservable = wsObservable.collect {
      case Binary(data) ⇒ data
    }

    WebsocketClient(binaryClient, binaryObservable, statusOutput)
  }
}
