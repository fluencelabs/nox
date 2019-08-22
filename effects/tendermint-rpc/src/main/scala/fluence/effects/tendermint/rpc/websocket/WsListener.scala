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

import cats.effect._
import cats.effect.concurrent.{Deferred, Ref}
import cats.effect.syntax.effect._
import cats.syntax.applicativeError._
import cats.syntax.apply._
import cats.syntax.compose._
import cats.syntax.either._
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.effects.tendermint.rpc.helpers.NettyFutureConversion._
import fluence.log.Log
import fs2.concurrent.Queue
import org.asynchttpclient.ws.{WebSocket, WebSocketListener}

import scala.language.higherKinds

/**
 * Listener for asynchttpclient's Websocket
 * @param wsUrl Websocket url, used for logging
 * @param payloadAccumulator Ref to collect message fragments
 * @param queue Queue to send resulting events to
 * @param disconnected Promise, will be completed when websocket signals disconnection
 */
class WsListener[F[_]: ConcurrentEffect](
  wsUrl: String,
  payloadAccumulator: Ref[F, String],
  queue: Queue[F, Event],
  disconnected: Deferred[F, WebsocketRpcError]
)(implicit log: Log[F])
    extends WebSocketListener {
  private val websocketP = Deferred.unsafe[F, WebSocket]

  /**
   * Callback for websocket opening, puts a Reconnect event to the queue
   */
  override def onOpen(websocket: WebSocket): Unit = {
    (log.info(s"Tendermint WRPC: $wsUrl connected") *>
      queue.enqueue1(Reconnect) >> websocketP.complete(websocket)).toIO.unsafeRunSync()
  }

  /**
   * Callback for websocket close, completes `disconnected` promise
   */
  override def onClose(websocket: WebSocket, code: Int, reason: String): Unit = {
    (log.warn(s"Tendermint WRPC: $wsUrl closed $code $reason") *>
      disconnected.complete(Disconnected(code, reason)).attempt.void).toIO.unsafeRunSync()
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
    // TODO: Add timeout on Ping
    websocketP.get
      .flatMap(_.sendPongFrame().asAsync.void)
      .toIO
      .runAsync {
        case Left(e) => log.error(s"Tendermint WRPC: $wsUrl ping failed: $e").toIO
        case _       => IO.unit
      }
      .unsafeRunSync()
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
