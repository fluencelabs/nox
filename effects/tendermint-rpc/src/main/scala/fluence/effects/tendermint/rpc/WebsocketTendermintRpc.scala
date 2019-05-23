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

package fluence.effects.tendermint.rpc

import cats.Applicative
import cats.data.EitherT
import cats.effect._
import cats.effect.concurrent.{Deferred, Ref}
import cats.effect.syntax.effect._
import cats.syntax.applicativeError._
import cats.syntax.compose._
import cats.syntax.either._
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.effects.JavaFutureConversion._
import fluence.effects.syntax.backoff._
import fluence.effects.syntax.eitherT._
import fluence.effects.tendermint.rpc.helpers.NettyFutureConversion._
import fluence.effects.{Backoff, EffectError}
import fs2.concurrent.Queue
import io.circe.Json
import org.asynchttpclient.Dsl._
import org.asynchttpclient.netty.ws.NettyWebSocket
import org.asynchttpclient.ws.{WebSocket, WebSocketListener, WebSocketUpgradeHandler}

import scala.language.higherKinds

/**
 * Implementation of Tendermint RPC Subscribe call
 * Details: https://tendermint.com/rpc/#subscribe
 */
trait WebsocketTendermintRpc extends slogging.LazyLogging {

  val host: String
  val port: Int

  private val wsUrl = s"ws://$host:$port/websocket"

  /**
   * Subscribe on new blocks from Tendermint
   */
  def subscribeNewBlock[F[_]: ConcurrentEffect: Timer]: fs2.Stream[F, Json] = {
    subscribe("NewBlock")
  }

  private def subscribe[F[_]: ConcurrentEffect: Timer](
    event: String
  ): fs2.Stream[F, Json] = {
    def subscribe(ws: WebSocket) = ws.sendTextFrame(request(event)).asAsync.void
    def cancelFiber(fiber: (Fiber[F, _], _)) = fiber._1.cancel

    fs2.Stream
      .bracket(for {
        queue <- Queue.unbounded[F, Json]
        // Connect in background forever, using same queue
        fiber <- Concurrent[F].start(connect(queue, subscribe))
      } yield (fiber, queue))(cancelFiber)
      .flatMap { case (_, queue) => queue.dequeue }

//    Resource.make {
//      for {
//        queue <- Queue.unbounded[F, Json]
//        // Connect in background forever, using same queue
//        fiber <- Concurrent[F].start(connect(queue, subscribe))
//      } yield (fiber, queue)
//    }(cancelFiber).map { case (_, queue) => queue.dequeue }
  }

  private def request(event: String) =
    s"""
       |{
       |    "jsonrpc": "2.0",
       |    "id": 1,
       |    "method": "subscribe",
       |    "params": [
       |        "tm.event = '$event'"
       |    ]
       |}
     """.stripMargin

  /**
   * Connects to Tendermint Websocket RPC, recreates socket on disconnect.
   * All received events are pushed to the queue. Async blocks until the end of the world.
   *
   * NOTE: this method is expected to be called in background (e.g., in a separate Fiber)
   * NOTE: events sent between reconnects WOULD BE LOST (this is OK for NewBlock though)
   *
   * @param queue Queue to send events to
   * @param onConnect Will be executed on each successful connection
   */
  private def connect[F[_]: ConcurrentEffect: Timer](
    queue: Queue[F, Json],
    onConnect: WebSocket => F[Unit]
  )(implicit bf: Backoff[WRpcError] = Backoff.default): F[Unit] = {
    def logConnectionError(e: EffectError) =
      Applicative[F].pure(logger.error(s"Tendermint WRPC: $wsUrl error connecting: ${e.getMessage}"))

    def close(ws: NettyWebSocket) = ws.sendCloseFrame().asAsync.attempt.void

    (for {
      // Ref to accumulate payload frames (websocket allows to split single message into several)
      ref <- Ref.of[F, String]("")
      // promise will be completed by exception when socket is disconnected
      promise <- Deferred[F, WRpcError]
      // keep connecting until success
      websocket <- Backoff.default.retry(socket(wsHandler(ref, queue, promise)), logConnectionError)
      _ <- onConnect(websocket)
      // wait until socket disconnects (it may never do)
      error <- promise.get
      // try to signal tendermint ws is closing ; TODO: will that ever succeed?
      _ <- close(websocket)
      _ = logger.info(s"Tendermint WRPC: $wsUrl will reconnect: ${error.getMessage}")
    } yield error.asLeft[Unit]).eitherT.backoff.void
  }

  private def socket[F[_]: Async](handler: WebSocketUpgradeHandler) =
    EitherT(
      Async[F].delay(
        asyncHttpClient()
          .prepareGet(wsUrl)
          .execute(handler)
          .toCompletableFuture
          .asAsync[F]
          .attempt
      ).flatten
    ).leftMap[EffectError](ConnectionFailed)

  private def wsHandler[F[_]: ConcurrentEffect](
    ref: Ref[F, String],
    queue: Queue[F, Json],
    disconnected: Deferred[F, WRpcError]
  ) =
    new WebSocketUpgradeHandler.Builder()
      .addWebSocketListener(
        new WebSocketListener {
          override def onOpen(websocket: WebSocket): Unit = {
            logger.info(s"Tendermint WRPC: $wsUrl connected")
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
                  case Right(json) => queue.enqueue1(json)
                } >> ref.set("")

              // TODO: run sync or async? which is better here? In examples, they do it async, but does it matter?
              processF.toIO.unsafeRunSync()
            }
          }

          override def onBinaryFrame(payload: Array[Byte], finalFragment: Boolean, rsv: Int): Unit = {
            logger.warn(s"UNIMPLEMENTED: Tendermint WRPC: $wsUrl unexpected binary frame")
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
      )
      .build()
}
