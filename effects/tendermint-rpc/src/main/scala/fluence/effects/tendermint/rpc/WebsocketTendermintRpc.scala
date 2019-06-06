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

import cats.{Applicative, Monad}
import cats.data.EitherT
import cats.effect._
import cats.effect.concurrent.{Deferred, Ref}
import cats.effect.syntax.effect._
import cats.syntax.applicativeError._
import cats.syntax.compose._
import cats.syntax.option._
import cats.syntax.applicative._
import cats.syntax.either._
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.effects.JavaFutureConversion._
import fluence.effects.syntax.backoff._
import fluence.effects.syntax.eitherT._
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.rpc.helpers.NettyFutureConversion._
import fluence.effects.{Backoff, EffectError}
import fluence.log.Log
import fs2.concurrent.Queue
import io.circe.Json
import org.asynchttpclient.Dsl._
import org.asynchttpclient.netty.ws.NettyWebSocket
import org.asynchttpclient.ws.{WebSocket, WebSocketListener, WebSocketUpgradeHandler}

import scala.language.higherKinds

sealed trait Event
case class JsonEvent(json: Json) extends Event
case object Reconnect extends Event

/**
 * Implementation of Tendermint RPC Subscribe call
 * Details: https://tendermint.com/rpc/#subscribe
 */
abstract class WebsocketTendermintRpc[F[_]: ConcurrentEffect: Timer: Monad] extends slogging.LazyLogging {
  self: TendermintRpc[F] =>

  val host: String
  val port: Int

  private val wsUrl = s"ws://$host:$port/websocket"

  /**
   * Subscribe on new blocks from Tendermint
   */
  def subscribeNewBlock(lastKnownHeight: Long)(
    implicit log: Log[F],
    backoff: Backoff[EffectError] = Backoff.default
  ): fs2.Stream[F, Block] = {
    // Start accepting and/or loading blocks from next to already-known block
    val startFrom = lastKnownHeight + 1

    fs2.Stream.eval(log.info(s"subscribeNewBlock lastKnownHeight $lastKnownHeight")) >>
      fs2.Stream.resource(subscribe("NewBlock")).flatMap { queue =>
        def loadBlock(height: Long): F[Block] =
          backoff.retry(self.block(height), e => log.error(s"load block $height", e))
        def parseBlock(json: Json, height: Long): F[Block] = backoff.retry(
          EitherT
            .fromEither[F](
              Block(json).leftMap[WebsocketRpcError](BlockParsingFailed(_, json.spaces2, height))
            )
            .recoverWith {
              case e =>
                log.info(s"error parsing log $height: $e")
                self.block(height).leftMap {
                  case RpcBlockParsingFailed(cause, raw, height) => BlockParsingFailed(cause, raw, height)
                  case rpcErr => BlockRetrievalError(rpcErr, height)
                }
            },
          e => log.error(s"parsing block $height", e)
        )

        queue.dequeue
          .evalMapAccumulate(startFrom) {
            // new block
            // the assumption here is that we never have `curHeight < parse(json).header.height`
            // all other cases are possible though. This should be guaranteed by block processing latching
            // (i.e., statemachine doesn't commit the next block until previous was processed)
            case (curHeight, JsonEvent(json)) =>
              log.info(s"new block. curHeight $curHeight") >>
                parseBlock(json, curHeight).flatMap(b => {
                  log.info(s"retrieved block curHeight $curHeight block ${b.header.height}").as {
                    // TODO: it could be only that `b.h.height == curHeight - 1`, check that, and log in other cases
                    if (b.header.height < curHeight) curHeight -> none[Block]
                    else (b.header.height + 1, b.some)
                  }
                })

            // reconnnect
            case (startHeight, Reconnect) =>
              for {
                _ <- log.info(s"reconnect. startHeight $startHeight")
                consensusHeight <- backoff
                  .retry(self.consensusHeight(), e => log.error("retrieving consensus height", e))
                _ <- log.info(s"reconnect. consensusHeight $consensusHeight")
                // since we're always maximum only 1 block behind (due to latching, see above), there're only 2 cases:
                (height, block) <- if (consensusHeight == startHeight)
                  // 1. startHeight == consensusHeight => lastKnownHeight == consensusHeight - 1, so we've missed 1 block
                  loadBlock(startHeight).map(b => (b.header.height + 1, b.some))
                else
                  // 2. startHeight == consensusHeight - 1 => lastKnownHeight == consensusHeight,
                  // so we're all caught up, and can start waiting for a new block (i.e., JsonEvent)
                  (startHeight, none[Block]).pure[F]
              } yield (height, block)
          }
          .map(_._2)
          .unNone
          .evalTap(b => log.info(s"subscription block ${b.header.height}"))
      }
  }

  private def subscribe(
    event: String,
  )(implicit backoff: Backoff[EffectError]): Resource[F, Queue[F, Event]] = {
    def subscribe(ws: WebSocket) = ws.sendTextFrame(request(event)).asAsync.void

    Resource
      .make(for {
        queue <- Queue.unbounded[F, Event]
        // Connect in background forever, using same queue
        fiber <- Concurrent[F].start(connect(queue, subscribe))
      } yield (fiber, queue))(_._1.cancel)
      .map { case (_, queue) => queue }
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
  private def connect(
    queue: Queue[F, Event],
    onConnect: WebSocket => F[Unit]
  )(implicit backoff: Backoff[EffectError]): F[Unit] = {
    def logConnectionError(e: EffectError) =
      Applicative[F].pure(logger.error(s"Tendermint WRPC: $wsUrl error connecting: ${e.getMessage}"))

    def close(ws: NettyWebSocket) = ws.sendCloseFrame().asAsync.attempt.void

    (for {
      // Ref to accumulate payload frames (websocket allows to split single message into several)
      ref <- Ref.of[F, String]("")
      // promise will be completed by exception when socket is disconnected
      promise <- Deferred[F, WebsocketRpcError]
      // keep connecting until success
      websocket <- backoff.retry(socket(wsHandler(ref, queue, promise)), logConnectionError)
      _ <- onConnect(websocket)
      // wait until socket disconnects (it may never do)
      error <- promise.get
      // try to signal tendermint ws is closing ; TODO: will that ever succeed?
      _ <- close(websocket)
      _ = logger.info(s"Tendermint WRPC: $wsUrl will reconnect: ${error.getMessage}")
    } yield (error: EffectError).asLeft[Unit]).eitherT.backoff.void
  }

  private def socket(handler: WebSocketUpgradeHandler) =
    EitherT(
      Async[F]
        .delay(
          asyncHttpClient()
            .prepareGet(wsUrl)
            .execute(handler)
            .toCompletableFuture
            .asAsync[F]
            .attempt
        )
        .flatten
    ).leftMap(ConnectionFailed)

  private def wsHandler(
    ref: Ref[F, String],
    queue: Queue[F, Event],
    disconnected: Deferred[F, WebsocketRpcError]
  ) =
    new WebSocketUpgradeHandler.Builder()
      .addWebSocketListener(
        new WebSocketListener {
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
      )
      .build()
}
