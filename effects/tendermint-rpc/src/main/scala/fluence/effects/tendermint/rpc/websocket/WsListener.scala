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
import cats.effect.concurrent.{Deferred, Ref}
import cats.effect.syntax.effect._
import cats.syntax.applicativeError._
import cats.syntax.compose._
import cats.syntax.either._
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.effects.tendermint.rpc.helpers.NettyFutureConversion._
import fs2.concurrent.Queue
import org.asynchttpclient.ws.{WebSocket, WebSocketListener}

import scala.language.higherKinds

class WsListener[F[_]: ConcurrentEffect](
  wsUrl: String,
  ref: Ref[F, String],
  queue: Queue[F, Event],
  disconnected: Deferred[F, WebsocketRpcError]
) extends WebSocketListener with slogging.LazyLogging {
  private val websocketP = Deferred.unsafe[F, WebSocket]

  override def onOpen(websocket: WebSocket): Unit = {
    logger.info(s"Tendermint WRPC: $wsUrl connected")
    (queue.enqueue1(Reconnect) >> websocketP.complete(websocket)).toIO.unsafeRunSync()
  }

  override def onClose(websocket: WebSocket, code: Int, reason: String): Unit = {
    logger.warn(s"Tendermint WRPC: $wsUrl closed $code $reason")
    disconnected.complete(Disconnected(code, reason)).attempt.void.toIO.unsafeRunSync()
  }

  override def onError(t: Throwable): Unit = {
    logger.error(s"Tendermint WRPC: $wsUrl $t")
    disconnected.complete(DisconnectedWithError(t)).attempt.void.toIO.unsafeRunSync()
  }

  override def onTextFrame(payload: String, finalFragment: Boolean, rsv: Int): Unit = {

    logger.trace(s"Tendermint WRPC: text $payload")
    if (!finalFragment) {
      ref.update(_.concat(payload)).toIO.unsafeRunSync()
    } else {
      val processF = ref.get
        .map(s => asJson(s.concat(payload)))
        .flatMap {
          case Left(e) =>
            Applicative[F].pure {
              logger.error(s"Tendermint WRPC: $wsUrl $e")
              logger.debug(s"Tendermint WRPC: $wsUrl $e err payload:\n" + payload)
            }
          case Right(json) => queue.enqueue1(JsonEvent(json))
        } >> ref.set("")

      // TODO: run sync or async? which is better here? In examples, they do it async, but does it matter?
      processF.toIO.unsafeRunSync()
    }
  }

  override def onBinaryFrame(payload: Array[Byte], finalFragment: Boolean, rsv: Int): Unit = {
    logger.warn(s"UNIMPLEMENTED: Tendermint WRPC: $wsUrl unexpected binary frame")
  }

  override def onPingFrame(payload: Array[Byte]): Unit = {
    websocketP.get.flatMap(_.sendPongFrame().asAsync.void).toIO.unsafeRunAsync {
      case Left(e) => logger.error(s"Tendermint WRPC: $wsUrl ping failed: $e")
      case _ =>
    }
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
