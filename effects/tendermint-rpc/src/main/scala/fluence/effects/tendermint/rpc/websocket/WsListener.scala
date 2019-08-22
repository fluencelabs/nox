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

import cats.{Functor, Monad}
import cats.effect._
import cats.effect.concurrent.{Deferred, MVar, Ref}
import cats.effect.syntax.effect._
import cats.syntax.applicativeError._
import cats.syntax.apply._
import cats.syntax.compose._
import cats.syntax.either._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.syntax.applicative._
import fluence.effects.tendermint.rpc.helpers.NettyFutureConversion._
import fluence.log.Log
import fs2.concurrent.Queue
import org.asynchttpclient.ws.{WebSocket, WebSocketListener}

import scala.concurrent.duration._
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
  pong: MVar[F, ()]
)(implicit log: Log[F])
    extends WebSocketListener {

  private val startPinging = {
    val period = 1.second
    val pingOrCloseAsync = {
      val timeout = 3.seconds
      // Send ping => receive pong => onPongFrame => pong.tryPut(())
      val sendPing = websocketP.get.flatMap(_.sendPingFrame().asAsync.void) >> log.debug("ping sent")
      // Clear pong of old pongs => send ping => wait for pong (pong could be a late one, but it's OK)
      val sendPingWaitPong = pong.tryTake >> sendPing >> pong.take >> log.debug("pong received")

      val close = websocketP.get >>= (this.close(_, code = None, s"no pong after timeout $timeout"))
      val closeAfterTimeout = Timer[F].sleep(timeout) >> close

      // Send ping, wait for pong or close websocket after timeout, all in background
      Concurrent[F].race(closeAfterTimeout, sendPingWaitPong)
    }

    ContextShift[F].shift >> pingOrCloseAsync.flatMap {
      // Timed out
      case Left(_) => ().asRight[Unit].pure[F]
      // Received pong, go ping again
      case Right(_) => Timer[F].sleep(period) >> pingOrCloseAsync
    }.void
  }

  /**
   * Callback for websocket opening, puts a Reconnect event to the queue
   */
  override def onOpen(websocket: WebSocket): Unit = {
    (log.info(s"Tendermint WRPC: $wsUrl connected") *>
      queue.enqueue1(Reconnect) >> websocketP.complete(websocket) >> startPinging).toIO.unsafeRunSync()
  }

  private def close(websocket: WebSocket, code: Option[Int], reason: String) = {
    log.warn(s"Tendermint WRPC: $wsUrl closed${code.getOrElse(" ")} $reason") *>
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
    (log.error(s"Tendermint WRPC: $wsUrl $t") *>
      disconnected.complete(DisconnectedWithError(t)).attempt.void).toIO.unsafeRunSync()
  }

  /**
   * Callback for receiving text payloads
   * @param payload Payload itself, could be just a fragment of a message
   * @param finalFragment Whether the payload is a final fragment of a message
   * @param rsv extension bits, not used here
   */
  override def onTextFrame(payload: String, finalFragment: Boolean, rsv: Int): Unit = {

    log.trace(s"Tendermint WRPC: text $payload")
    if (!finalFragment) {
      payloadAccumulator.update(_.concat(payload)).toIO.unsafeRunSync()
    } else {
      val processF = payloadAccumulator.get
        .map(s => asJson(s.concat(payload)))
        .flatMap {
          case Left(e) =>
            log.error(s"Tendermint WRPC: $wsUrl $e") *>
              log.debug(s"Tendermint WRPC: $wsUrl $e err payload:\n" + payload)
          case Right(json) => queue.enqueue1(JsonEvent(json))
        } >> payloadAccumulator.set("")

      // TODO: run sync or async? which is better here? In examples, they do it async, but does it matter?
      processF.toIO.unsafeRunSync()
    }
  }

  override def onBinaryFrame(payload: Array[Byte], finalFragment: Boolean, rsv: Int): Unit = {
    log.warn(s"UNIMPLEMENTED: Tendermint WRPC: $wsUrl unexpected binary frame").toIO.unsafeRunSync()
  }

  /**
   * Callback for pings, sends back a Pong message
   * @param payload Ping payload
   */
  override def onPingFrame(payload: Array[Byte]): Unit = {
    val sendPong = websocketP.get >>= (_.sendPingFrame().asAsync.void)

    sendPong
      .toIO
      .runAsync {
        case Left(e) => log.error(s"Tendermint WRPC: $wsUrl ping failed: $e").toIO
        case _       => IO.unit
      }
      .unsafeRunSync()
  }

  override def onPongFrame(payload: Array[Byte]): Unit = {
    pong.tryPut(()).toIO.unsafeRunSync()
  }

  private def asJson(payload: String) = {
    import io.circe._
    import io.circe.parser._

    // TODO: handle errors
    // TODO: handle response on subscribe ("result": {}) (- _  -)
    for {
      json <- parse(payload).leftMap(InvalidJsonResponse)
      valueJson <- json.hcursor
        .downField("result")
        .downField("data")
        .downField("value")
        .as[Json]
        .leftMap(InvalidJsonStructure)
    } yield valueJson
  }
}

object WsListener {

  def apply[F[_]: ConcurrentEffect: Timer: ContextShift](
    wsUrl: String,
    payloadAccumulator: Ref[F, String],
    queue: Queue[F, Event],
    disconnected: Deferred[F, WebsocketRpcError]
  )(implicit log: Log[F]): F[WsListener[F]] =
    for {
      _ <- Monad[F].pure()
      wsListener = new WsListener(wsUrl, payloadAccumulator, queue, disconnected, ???, ???)
    } yield wsListener
}
