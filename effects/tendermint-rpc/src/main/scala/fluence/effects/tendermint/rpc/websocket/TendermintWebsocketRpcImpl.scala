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

import cats.data.EitherT
import cats.effect._
import cats.effect.concurrent.{Deferred, Ref}
import cats.instances.list._
import cats.syntax.applicative._
import cats.syntax.applicativeError._
import cats.syntax.apply._
import cats.syntax.either._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.{Eval, Monad, Traverse}
import fluence.effects.JavaFutureConversion._
import fluence.effects.syntax.backoff._
import fluence.effects.syntax.eitherT._
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.rpc.helpers.NettyFutureConversion._
import fluence.effects.tendermint.rpc.http.{RpcBlockParsingFailed, TendermintHttpRpc}
import fluence.effects.{Backoff, EffectError}
import fluence.log.Log
import fs2.concurrent.Queue
import io.circe.Json
import org.asynchttpclient.Dsl._
import org.asynchttpclient.netty.ws.NettyWebSocket
import org.asynchttpclient.ws.{WebSocket, WebSocketUpgradeHandler}

import scala.language.higherKinds

private[websocket] sealed trait Event
private[websocket] case class JsonEvent(json: Json) extends Event
private[websocket] case object Reconnect extends Event

/**
 * Implementation of Tendermint RPC Subscribe call
 * Details: https://tendermint.com/rpc/#subscribe
 */
abstract class TendermintWebsocketRpcImpl[F[_]: ConcurrentEffect: Timer: Monad: ContextShift]
    extends TendermintWebsocketRpc[F] {
  self: TendermintHttpRpc[F] =>

  val host: String
  val port: Int

  private val wsUrl = s"ws://$host:$port/websocket"

  /**
   * Subscribe on new blocks from Tendermint, retrieves missing blocks and keeps them in order
   *
   * @param lastKnownHeight Height of the block that was already processed (uploaded, and its receipt stored)
   * @return Stream of blocks, strictly in order, without any repetitions
   */
  def subscribeNewBlock(lastKnownHeight: Long)(
    implicit log: Log[F],
    backoff: Backoff[EffectError] = Backoff.default
  ): fs2.Stream[F, Block] = {
    // Start accepting and/or loading blocks from next to already-known block
    val startFrom = lastKnownHeight + 1

    fs2.Stream.resource(subscribe("NewBlock")).flatMap { queue =>
      fs2.Stream.eval(traceBU(s"subscribed on NewBlock. startFrom: $startFrom")) *>
        queue.dequeue
          .evalMapAccumulate(startFrom) {
            // load missing blocks on reconnect (reconnect is always the first event in the queue)
            case (startHeight, Reconnect) => loadMissedBlocks(startHeight)
            // accept a new block
            case (curHeight, JsonEvent(json)) => acceptNewBlock(curHeight, json)
          }
          .flatMap { case (_, blocks) => fs2.Stream.emits(blocks) }
    }
  }

  /**
   * Accepts a new block.
   *
   *  If block is higher than expected height, loads missing blocks
   *  If block is lower than expected height, ignores that block
   *  Otherwise block is accepted, and the expected height is incremented
   *
   * @param expectedHeight Expected block height
   * @param blockJson Block encoded in json
   * @return Tuple of new expected height, and resulting blocks (could be 0 or more)
   */
  private def acceptNewBlock(expectedHeight: Long, blockJson: Json)(
    implicit log: Log[F],
    backoff: Backoff[EffectError]
  ) =
    parseBlock(blockJson, expectedHeight)
      .flatTap(b => traceBU(s"new block ${b.header.height}. expectedHeight $expectedHeight"))
      .flatMap {
        // received an old block, ignoring
        case b if b.header.height < expectedHeight =>
          log.warn(s"ignoring block ${b.header.height} as too old, current height is $expectedHeight") as
            expectedHeight -> List.empty[Block]
        // we've missed some blocks, so catching up (this happened without reconnect, so it might be Tendermint's error)
        case b if b.header.height > expectedHeight =>
          for {
            _ <- log.warn(s"missed some blocks. expected $expectedHeight, got ${b.header.height}. catching up")
            blocks <- loadBlocks(expectedHeight, b.header.height - 1)
          } yield (b.header.height + 1, blocks :+ b)
        case b =>
          (b.header.height + 1, List(b)).pure[F]
      }

  /**
   * Loads missing blocks if there are any
   * @param startHeight Height to load blocks from. If greater than current consensus height - does nothing
   * @return Tuple of next expected height and resulting blocks (could be 0 or more blocks)
   */
  private def loadMissedBlocks(startHeight: Long)(
    implicit log: Log[F],
    backoff: Backoff[EffectError]
  ) = {
    def warnIf(cond: => Boolean, msg: String) = if (cond) log.warn(msg) else ().pure[F]
    for {
      // retrieve height from Tendermint
      consensusHeight <- backoff.retry(self.consensusHeight(), e => log.error("retrieving consensus height", e))
      _ <- traceBU(
        s"reconnect. startHeight $startHeight consensusHeight $consensusHeight " +
          s"cond1: ${consensusHeight == startHeight}, cond2: ${startHeight == consensusHeight - 1}"
      )
      (height, block) <- if (consensusHeight >= startHeight) {
        // we're behind last block, load all blocks up to it
        loadBlocks(startHeight, consensusHeight).map(bs => (consensusHeight + 1, bs))
      } else {
        warnIf(
          // shouldn't happen, could mean that we have invalid blocks saved in storage
          startHeight > consensusHeight + 1,
          s"unexpected state: startHeight $startHeight > consensusHeight $consensusHeight + 1. " +
            s"Consensus travelled back in time?"
        ) as
          // we're all caught up, start waiting for a new block (i.e., JsonEvent)
          (startHeight, List.empty[Block])
      }
    } yield (height, block)
  }

  /**
   * Parses block json. Loads the block at the height if json is incorrect.
   * @param json Block encoded in json
   * @param height Expected height of the block
   * @return Parsed or loaded block
   */
  private def parseBlock(json: Json, height: Long)(
    implicit log: Log[F],
    backoff: Backoff[EffectError]
  ): F[Block] = backoff.retry(
    EitherT
      .fromEither[F](
        Block(json).leftMap[WebsocketRpcError](BlockParsingFailed(_, Eval.later(json.spaces2), height))
      )
      .recoverWith {
        case e =>
          Log.eitherT[F, WebsocketRpcError].warn(s"parsing block $height, reloading", e) >>
            self.block(height).leftMap {
              case RpcBlockParsingFailed(cause, raw, height) => BlockParsingFailed(cause, Eval.now(raw), height)
              case rpcErr                                    => BlockRetrievalError(rpcErr, height)
            }
      },
    e => log.error(s"parsing block $height", e)
  )

  private def loadBlock(height: Long)(
    implicit log: Log[F],
    backoff: Backoff[EffectError]
  ): F[Block] = backoff.retry(self.block(height), e => log.error(s"load block $height", e))

  private def loadBlocks(from: Long, to: Long)(
    implicit log: Log[F],
    backoff: Backoff[EffectError]
  ) = Traverse[List].sequence((from to to).map(loadBlock).toList)

  /**
   * Subscribes to the specified event type
   * @param event Event type
   * @return Queue of events
   */
  protected def subscribe(
    event: String,
  )(implicit log: Log[F], backoff: Backoff[EffectError]): Resource[F, Queue[F, Event]] = {
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
  )(implicit backoff: Backoff[EffectError], log: Log[F]): F[Unit] = {
    def logConnectionError(e: EffectError) =
      log.error(s"Tendermint WRPC: $wsUrl error connecting: ${e.getMessage}")

    def close(ws: NettyWebSocket) = ws.sendCloseFrame().asAsync.attempt.void

    Monad[F].tailRecM(())(
      _ =>
        for {
          // Ref to accumulate payload frames (websocket allows to split single message into several)
          messageAccumulator <- Ref.of[F, String]("")
          // promise will be completed by exception when socket is disconnected
          promise <- Deferred[F, WebsocketRpcError]
          // keep connecting until success
          connectSocket = wsHandler(messageAccumulator, queue, promise) >>= socket
          _ <- log.debug(s"Tendermint WRPC: $wsUrl started connecting")
          websocket <- backoff.retry(connectSocket, logConnectionError)
          _ <- onConnect(websocket)
          // wait until socket disconnects (it may never do)
          error <- promise.get
          // signal tendermint ws is closing
          _ <- close(websocket)
          _ <- log.info(s"Tendermint WRPC: $wsUrl will reconnect: ${error.getMessage}")
        } yield ().asLeft // keep tailRecM calling this forever
    )
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
    payloadAccumulator: Ref[F, String],
    queue: Queue[F, Event],
    disconnected: Deferred[F, WebsocketRpcError]
  )(implicit log: Log[F]): EitherT[F, ConnectionFailed, WebSocketUpgradeHandler] =
    EitherT.liftF(
      WsListener[F](wsUrl, payloadAccumulator, queue, disconnected, websocketConfig).map(
        new WebSocketUpgradeHandler.Builder()
          .addWebSocketListener(_)
          .build()
      )
    )

  // Writes a trace log about block uploading
  private def traceBU(msg: String)(implicit log: Log[F]) =
    log.trace(Console.YELLOW + s"BUD: $msg" + Console.RESET)
}
