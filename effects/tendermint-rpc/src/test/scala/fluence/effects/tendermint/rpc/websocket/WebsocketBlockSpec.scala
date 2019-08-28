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
import cats.effect.concurrent.Ref
import cats.effect.{IO, Resource}
import cats.syntax.either._
import cats.syntax.functor._
import fluence.effects.sttp.SttpEffect
import fluence.effects.syntax.eitherT._
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.rpc
import fluence.effects.tendermint.rpc.http.{RpcError, RpcRequestFailed}
import fluence.effects.{Backoff, EffectError}
import fluence.log.{Log, LogFactory}
import fs2.concurrent.Queue
import io.circe.Json
import io.circe.parser.parse
import org.scalatest.{Matchers, OptionValues, WordSpec}

import scala.concurrent.ExecutionContext.Implicits.global
import scala.concurrent.duration._
import scala.language.postfixOps

class WebsocketBlockSpec extends WordSpec with Matchers with OptionValues {
  implicit private val timer = IO.timer(global)
  implicit private val shift = IO.contextShift(global)
  implicit private val log = LogFactory.forPrintln[IO]().init("block uploading spec", level = Log.Error).unsafeRunSync()
  implicit private val sttp = SttpEffect.plain[IO]

  sealed trait Action
  case object GetConsensusHeight extends Action
  case class GetBlock(height: Long) extends Action
  case class ExecutionState(actions: List[Action] = Nil) {
    def block(height: Long) = copy(actions :+ GetBlock(height))
    def consensusHeight() = copy(actions :+ GetConsensusHeight)
  }

  private def blockJson(height: Long): Json = {
    parse(rpc.TestData.block(height)).right.get.hcursor
      .downField("result")
      .downField("data")
      .get[Json]("value")
      .right
      .get
  }

  private def singleBlock(height: Long) = {
    Block(blockJson(height)).right.get
  }

  def websocket(heights: List[Long], events: List[Event]) = {
    def ws(consensusHeights: Ref[IO, List[Long]], events: List[Event], state: Ref[IO, ExecutionState]) =
      new TendermintWebsocketRpcImpl[IO] with TestTendermintRpc[IO] {
        override val websocketConfig: WebsocketConfig = WebsocketConfig()
        override val host: String = "WebsocketBlockSpecNonExistingHost"
        override val port: Int = 3333333

        override def block(height: Long, id: String): EitherT[IO, RpcError, Block] =
          (for {
            _ <- state.update(_.block(height))
          } yield singleBlock(height).asRight[RpcError]).eitherT

        override def consensusHeight(id: String): EitherT[IO, RpcError, Long] =
          (for {
            _ <- state.update(_.consensusHeight())
            height <- consensusHeights.modify(l => (l.tail, l.headOption))
          } yield height.fold(
            (RpcRequestFailed(
              new Throwable("WebSocketBlockSpec: requested consensus height when consensusHeights list is empty")
            ): RpcError).asLeft[Long]
          )(_.asRight)).eitherT

        override protected def subscribe(
          event: String
        )(implicit log: Log[IO], backoff: Backoff[EffectError]): Resource[IO, Queue[IO, Event]] =
          Resource.liftF(fs2.concurrent.Queue.unbounded[IO, Event]).evalMap { q =>
            fs2.Stream.emits(events).through(q.enqueue).compile.drain.as(q)
          }
      }

    for {
      consensusHeights <- Ref.of[IO, List[Long]](heights)
      state <- Ref.of[IO, ExecutionState](ExecutionState())
      w = ws(consensusHeights, events, state)
    } yield (w, state)
  }

  private def emitBlocks(
    lastKnownHeight: Long,
    consensusHeights: List[Long],
    events: List[Event],
    expectedBlocks: List[Long],
    expectedActions: List[Action],
    timeout: Duration = 10.seconds
  ) = {
    val result = (websocket(consensusHeights, events).flatMap {
      case (ws, state) =>
        ws.subscribeNewBlock(lastKnownHeight)
          .take(expectedBlocks.length)
          .compile
          .toList
          .flatMap(bs => state.get.map(bs -> _))
    }).unsafeRunTimed(timeout)

    result match {
      case None => fail(s"Test timed out after $timeout")
      case Some((blocks, state)) =>
        blocks.map(_.header.height) should contain theSameElementsInOrderAs expectedBlocks
        state.actions should contain theSameElementsInOrderAs expectedActions
        state.actions.count(_ == GetConsensusHeight) shouldBe consensusHeights.length
    }
  }

  def block(height: Long) = JsonEvent(blockJson(height))

  "websocket" should {

    "yield blocks in order" in {
      // given lastKnownHeight = 1
      // and consensusHeight = 2
      // it should
      // loadBlock(2)
      // emit blocks 3, 4

      val lastKnownHeight = 1
      val consensusHeight = 2
      val blocks = (3 to 4).map(blockJson(_)) toList
      val events = Reconnect +: blocks.map(JsonEvent)
      val expectedBlocks = List(2L, 3L, 4L)
      val expectedActions = List(GetConsensusHeight, GetBlock(2))

      emitBlocks(lastKnownHeight, List(consensusHeight), events, expectedBlocks, expectedActions)
    }

    "load block after reconnect" in {
      // lastKnownHeight = 1
      // consensusHeight = 2
      // loadBlock(2)
      // emit block = 3
      // emit reconnect
      // consensusHeight = 4
      // loadBlock(4)
      // emit blocks 4, 4, 5

      val lastKnownHeight = 1
      val consensusHeights = List(2L, 4L)
      val events = List(Reconnect, block(3), Reconnect, block(4), block(4), block(5))
      val expectedBlocks = List(2L, 3L, 4L, 5L)
      val expectedActions = List(GetConsensusHeight, GetBlock(2), GetConsensusHeight, GetBlock(4))

      emitBlocks(lastKnownHeight, consensusHeights, events, expectedBlocks, expectedActions)
    }

    "not load block after reconnect" in {
      // lastKnownHeight = 1
      // consensusHeight = 2
      // loadBlock(2)
      // emit block = 3
      // emit reconnect
      // consensusHeight = 3
      // emit block = 4

      val lastKnownHeight = 1
      val consensusHeights = List(2L, 3L)
      val events = List(Reconnect, block(3), Reconnect, block(4))
      val expectedBlocks = List(2L, 3L, 4L)
      val expectedActions = List(GetConsensusHeight, GetBlock(2), GetConsensusHeight)

      emitBlocks(lastKnownHeight, consensusHeights, events, expectedBlocks, expectedActions)
    }

    "ignore old blocks" in {
      // emit blocks 1 x3, 2 x3, 3, 2 x3, 3 x3, 4, 3, 2, 1

      def blockx3(height: Long) = (1 to 3).map(_ => block(height))

      val lastKnownHeight = 0
      val consensusHeights = List(0L)
      val blocks = blockx3(1) ++ (blockx3(2) :+ block(3)) ++ blockx3(2) ++ blockx3(3) ++ (1L to 4L).reverse.map(block)
      val events: List[Event] = Reconnect +: blocks toList
      val expectedBlocks = List(1L, 2L, 3L, 4L)
      val expectedActions = List(GetConsensusHeight)

      emitBlocks(lastKnownHeight, consensusHeights, events, expectedBlocks, expectedActions)
    }

    "load missing blocks" in {
      val lastKnownHeight = 0
      val consensusHeights = List(3L, 7L)
      val events = List(
        // start and load blocks from 1 to 3
        Reconnect,
        // blocks 2 and 3 will be ignored
        block(2),
        block(3),
        // ordinary blocks
        block(4),
        block(5),
        // load blocks 6 and 7
        Reconnect,
        // ordinary blocks
        block(7),
        block(8)
      )
      val expectedBlocks = List(1L, 2L, 3L, 4L, 5L, 6L, 7L, 8L)
      val expectedActions =
        List(GetConsensusHeight, GetBlock(1), GetBlock(2), GetBlock(3), GetConsensusHeight, GetBlock(6), GetBlock(7))

      emitBlocks(lastKnownHeight, consensusHeights, events, expectedBlocks, expectedActions)
    }

    "load missing blocks without reconnect" in {
      val lastKnownHeight = 0
      val consensusHeights = List(0L)
      val events = List(Reconnect, block(3), block(4), block(7))
      val expectedBlocks = (1L to 7L).toList
      val expectedActions = List(GetConsensusHeight, GetBlock(1), GetBlock(2), GetBlock(5), GetBlock(6))

      emitBlocks(lastKnownHeight, consensusHeights, events, expectedBlocks, expectedActions)
    }
  }
}
