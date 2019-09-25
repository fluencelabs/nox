/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package fluence.effects.tendermint.rpc.websocket

import cats.Applicative
import cats.effect._
import cats.effect.concurrent.{Deferred, MVar, Ref}
import cats.effect.syntax.effect._
import cats.syntax.applicative._
import cats.syntax.applicativeError._
import cats.syntax.compose._
import cats.syntax.either._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.syntax.option._
import fluence.effects.tendermint.rpc.helpers.NettyFutureConversion._
import fluence.log.Log
import fs2.concurrent.Queue
import io.circe.Json
import org.asynchttpclient.ws.{WebSocket, WebSocketListener}

import scala.language.higherKinds

/**
 * Listener for asynchttpclient's Websocket
 * @param wsUrl Websocket url, used for logging
 * @param payloadAccumulator Ref to collect message fragments
 * @param queue Queue to send resulting events to
 * @param disconnected Promise, will be completed when websocket signals disconnection
 */
class WsListener[F[_]: ConcurrentEffect: Timer: ContextShift](
  wsUrl: String,
  payloadAccumulator: Ref[F, String],
  queue: Queue[F, Event],
  disconnected: Deferred[F, WebsocketRpcError],
  websocketP: Deferred[F, WebSocket],
  pong: MVar[F, Unit],
  config: WebsocketConfig
)(implicit log: Log[F])
    extends WebSocketListener {

  private def startPinging(ws: WebSocket) = {
    import config.{pingInterval, pingTimeout => timeout}

    def pingOrCloseAsync: F[Unit] = {
      val sendPing = ws.sendPingFrame().asConcurrent
      val sendPingWaitPong = pong.tryTake >> sendPing >> pong.take

      val close = this.close(ws, code = None, s"no pong after timeout $timeout")
      val closeAfterTimeout = Timer[F].sleep(timeout) >> close

      // Send ping, wait for pong or close websocket after timeout, all in background
      Concurrent[F].race(closeAfterTimeout, sendPingWaitPong)
    }.flatMap {
      case Left(_)  => ().pure[F]
      case Right(_) => Timer[F].sleep(pingInterval) >> pingOrCloseAsync
    }.void

    def stopPinging(fiber: Fiber[F, _]) = Concurrent[F].start(disconnected.get >> fiber.cancel)

    Concurrent[F].start(pingOrCloseAsync).flatMap(stopPinging).void
  }

  /**
   * Callback for websocket opening, puts a Reconnect event to the queue
   */
  override def onOpen(websocket: WebSocket): Unit = {
    log.info(s"Tendermint WRPC: $wsUrl connected") >>
      queue.enqueue1(Reconnect) >> websocketP.complete(websocket) >> startPinging(websocket)
  }.toIO.unsafeRunSync()

  private def close(websocket: WebSocket, code: Option[Int], reason: String) = {
    log.warn(s"Tendermint WRPC: $wsUrl closed ${code.getOrElse(" ")} $reason") >>
      disconnected.complete(Disconnected(code, reason)).attempt.void
  }

  /**
   * Callback for websocket close, completes `disconnected` promise
   */
  override def onClose(websocket: WebSocket, code: Int, reason: String): Unit = {
    close(websocket, Some(code), reason).toIO.unsafeRunSync()
  }

  /**
   * Callback for errors, completes `disconnected` promise
   */
  override def onError(t: Throwable): Unit = {
    log.error(s"Tendermint WRPC: $wsUrl $t") >>
      disconnected.complete(DisconnectedWithError(t)).attempt.void
  }.toIO.unsafeRunSync()

  /**
   * Callback for receiving text payloads
   * @param payload Payload itself, could be just a fragment of a message
   * @param finalFragment Whether the payload is a final fragment of a message
   * @param rsv extension bits, not used here
   */
  override def onTextFrame(payload: String, finalFragment: Boolean, rsv: Int): Unit = {
    if (!finalFragment) {
      payloadAccumulator.update(_.concat(payload))
    } else {
      payloadAccumulator.get
        .map(s => asJson(s.concat(payload)))
        .flatMap {
          case Left(e) =>
            log.error(s"Tendermint WRPC: $wsUrl $e") >>
              log.trace(s"Tendermint WRPC: $wsUrl $e err payload:\n" + payload)
          case Right(Some(json)) => queue.enqueue1(JsonEvent(json))
          case Right(None)       => Applicative[F].unit
        } >> payloadAccumulator.set("")

    }
  }.toIO.unsafeRunSync()

  override def onBinaryFrame(payload: Array[Byte], finalFragment: Boolean, rsv: Int): Unit =
    log.warn(s"UNIMPLEMENTED: Tendermint WRPC: $wsUrl unexpected binary frame").toIO.unsafeRunSync()

  /**
   * Callback for pings, sends back a Pong message
   * @param payload Ping payload
   */
  override def onPingFrame(payload: Array[Byte]): Unit = {
    val sendPong = websocketP.get >>= (_.sendPongFrame().asConcurrent.void)

    sendPong.toIO.runAsync {
      case Left(e) => log.error(s"Tendermint WRPC: $wsUrl ping failed: $e").toIO
      case _       => IO.unit
    }.unsafeRunSync()
  }

  override def onPongFrame(payload: Array[Byte]): Unit = {
    pong.tryPut(()).toIO.unsafeRunSync()
  }

  /**
   * Parses payload as json
   *
   * @return Error, Json or None if payload didn't contain meaningful data – i.e. response on subscription
   */
  private def asJson(payload: String): Either[WebsocketRpcError, Option[Json]] = {
    import io.circe._
    import io.circe.parser._

    // TODO: handle errors
    for {
      json <- parse(payload).leftMap(InvalidJsonResponse)
      data = json.hcursor.downField("result").downField("data")
      valueJson <- data.success.fold(none[Json].asRight[WebsocketRpcError])(
        _.downField("value").as[Json].leftMap(InvalidJsonStructure).map(_.some)
      )
    } yield valueJson
  }
}

object WsListener {

  def apply[F[_]: ConcurrentEffect: Timer: ContextShift](
    wsUrl: String,
    payloadAccumulator: Ref[F, String],
    queue: Queue[F, Event],
    disconnected: Deferred[F, WebsocketRpcError],
    config: WebsocketConfig
  )(implicit log: Log[F]): F[WsListener[F]] =
    for {
      websocketP <- Deferred[F, WebSocket]
      pong <- MVar.empty[F, Unit]
      wsListener = new WsListener(wsUrl, payloadAccumulator, queue, disconnected, websocketP, pong, config)
    } yield wsListener
}
