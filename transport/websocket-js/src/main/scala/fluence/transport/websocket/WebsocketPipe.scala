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

import fluence.codec.PureCodec
import cats.instances.future._
import monix.execution.Ack.{Continue, Stop}
import monix.execution.{Ack, Scheduler}
import monix.reactive._

import scala.concurrent.duration.FiniteDuration
import scala.concurrent.{ExecutionContext, Future, Promise}
import scala.concurrent.duration._
import scala.language.higherKinds

//TODO add error field in WebsocketMessage and Either in WebsocketPipe to handle and return errors between nodes
case class WebsocketPipe[A, B] private (
  input: Observer[A],
  output: Observable[B],
  statusOutput: Observable[StatusFrame]
) extends slogging.LazyLogging {

  /**
   * Convert WebsocketPipe to another one.
   *
   * @param inputCodec Codec to converting input elements.
   * @param outputCodec Condec to converting output elements.
   * @tparam A1 Type of new input.
   * @tparam B1 Type on new output.
   * @return New WebsocketPipe.
   */
  def xmap[A1, B1](inputCodec: PureCodec.Func[A1, A], outputCodec: PureCodec.Func[B, B1])(
    implicit ec: ExecutionContext
  ): WebsocketPipe[A1, B1] = {

    val tObserver: Observer[A1] = new Observer[A1] {
      override def onNext(elem: A1): Future[Ack] = {
        inputCodec
          .runF[Future](elem)
          .flatMap(input.onNext)
          .recover {
            case e: Throwable ⇒
              //TODO add either and send error message
              logger.error(s"Error on converting message $elem with $inputCodec", e)
              Continue
          }
      }

      override def onError(ex: Throwable): Unit = input.onError(ex)

      override def onComplete(): Unit = input.onComplete()
    }

    val tObservable = output.mapFuture { el ⇒
      //TODO add either or observable will stop now after error
      outputCodec.runF[Future](el)
    }

    WebsocketPipe(tObserver, tObservable, statusOutput)
  }

  /**
   * Send request to websocket and wait one result. For unary call.
   * @param request Request to send.
   * @return Response in future.
   */
  def requestAndWaitOneResult(request: A)(implicit scheduler: Scheduler): Future[B] = {
    val result: Promise[B] = Promise[B]

    val obs = new Observer[B] {

      override def onNext(elem: B): Future[Ack] = {
        result.success(elem)
        Future(Stop)
      }

      override def onError(ex: Throwable): Unit = {
        result.failure(ex)
        onComplete()
      }

      override def onComplete(): Unit = ()
    }

    output.subscribe(obs)

    input.onNext(request)

    result.future
  }
}

object WebsocketPipe {

  /**
   * Type to simplify pipes with equals request and response.
   */
  type WebsocketClient[T] = WebsocketPipe[T, T]

  /**
   * Creates WebsocketPipe.
   *
   * @param url Address to connect by websocket.
   * @param builder Builder for native websocket client.
   * @param numberOfAttempts Number for attempts to try reconnect websocket.
   * @param connectTimeout Timeout between reconnections.
   * @return An observer that will be an input into a websocket and observable - output
   */
  def apply(
    url: String,
    builder: String ⇒ WebsocketT,
    numberOfAttempts: Int,
    connectTimeout: FiniteDuration
  )(
    implicit scheduler: Scheduler
  ): WebsocketClient[WebsocketFrame] = {

    val queueingSubject = new SubjectQueue[WebsocketFrame]

    val (statusInput, statusOutput) = Observable.multicast(MulticastStrategy.publish[StatusFrame])

    val observable =
      new WebsocketObservable(url, builder, queueingSubject, statusInput, numberOfAttempts, connectTimeout)

    val hotObservable = observable.multicast(Pipe.publish[WebsocketFrame])
    hotObservable.connect()

    WebsocketPipe(queueingSubject, hotObservable, statusOutput)
  }

  /**
   * Client that accepts only binary data.
   */
  def binaryClient(
    url: String,
    builder: String ⇒ WebsocketT,
    numberOfAttempts: Int = 3,
    connectTimeout: FiniteDuration = 3.seconds
  )(implicit scheduler: Scheduler): WebsocketClient[Array[Byte]] = {

    val WebsocketPipe(wsObserver, wsObservable, statusOutput) =
      WebsocketPipe(url, builder, numberOfAttempts, connectTimeout)

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
