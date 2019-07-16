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
import fluence.EitherTSttpBackend
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.rpc
import fluence.effects.tendermint.rpc.http.{RpcError, RpcRequestFailed}
import fluence.effects.{Backoff, EffectError}
import fluence.log.{Log, LogFactory}
import fs2.concurrent.Queue
import io.circe.Json
import io.circe.parser.parse
import org.scalatest.{Matchers, OptionValues, WordSpec}
import cats.syntax.either._
import cats.syntax.applicative._
import cats.syntax.apply._
import cats.syntax.functor._
import cats.syntax.option._
import cats.syntax.either._
import fluence.effects.syntax.eitherT._

import scala.concurrent.ExecutionContext.Implicits.global
import scala.concurrent.duration._
import scala.language.postfixOps

class WebsocketBlockSpec extends WordSpec with Matchers with OptionValues {
  implicit private val timer = IO.timer(global)
  implicit private val shift = IO.contextShift(global)
  implicit private val log = LogFactory.forPrintln[IO]().init("block uploading spec", level = Log.Warn).unsafeRunSync()
  implicit private val sttp = EitherTSttpBackend[IO]()

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
          } yield
            height.fold(
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

  def emitBlocks(lastKnownHeight: Long,
                 events: List[Event],
                 expectedBlocks: List[Long],
                 expectedActions: List[Action],
                 timeout: Duration = 10.seconds) = {
    val result = (websocket(List(2), events).flatMap {
      case (ws, state) =>
        ws.subscribeNewBlock(lastKnownHeight).take(3).compile.toList.flatMap(bs => state.get.map(bs -> _))
    }).unsafeRunTimed(timeout)

    result match {
      case None => fail(s"Test timed out after $timeout")
      case Some((blocks, state)) =>
        blocks.length shouldBe expectedBlocks.length
        blocks.map(_.header.height) should contain theSameElementsInOrderAs expectedBlocks

        state.actions.length shouldBe expectedActions.length
        state.actions should contain theSameElementsInOrderAs expectedActions
    }
  }

  "websocket" should {
    "yield blocks in order" in {
      // lastKnownHeight = 1
      // consensusHeight = 2
      // loadBlock(2)
      // emit block = 3
      // emit block = 4

      val blocks = (3 to 4).map(blockJson(_)) toList
      val events = Reconnect +: blocks.map(JsonEvent)
      val expectedBlocks = List(2L, 3L, 4L)
      val expectedActions = List(GetConsensusHeight, GetBlock(2))

      emitBlocks(1, events, expectedBlocks, expectedActions)
    }

    "load block after reconnect" in {
      // lastKnownHeight = 1
      // consensusHeight = 2
      // loadBlock(2)
      // emit block = 3
      // emit reconnect
      // consensusHeight = 4
      // loadBlock(4)
      // emit block 4
      // emit block 4
      // emit block 5
    }

    "not load block after reconnect" in {
      // lastKnownHeight = 1
      // consensusHeight = 2
      // loadBlock(2)
      // emit block = 3
      // emit reconnect
      // consensusHeight = 3
      // emit block = 4
    }

    "ignore old blocks" in {
      // emit block 4
      // emit block 4
      // emit block 5
      // emit block 5
      // emit block 6
    }

    "load missed blocks" in {
      // emit block 4
      // emit block 7
      // loadBlock(5)
      // loadBlock(6)
    }
  }
}
