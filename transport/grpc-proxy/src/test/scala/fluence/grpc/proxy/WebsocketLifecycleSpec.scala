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

package fluence.grpc.proxy

import fluence.proxy.grpc.WebsocketMessage
import fluence.proxy.grpc.WebsocketMessage.Response
import org.scalatest.{Assertion, AsyncWordSpec, Matchers}
import fs2._
import monix.eval.Task
import monix.execution.Ack
import org.http4s.websocket.WebsocketBits.{Binary, Ping, WebSocketFrame}
import monix.execution.Scheduler.Implicits.global
import monix.reactive.{Observable, Observer}

import scala.concurrent.Promise
import scala.util.Random

class WebsocketLifecycleSpec extends AsyncWordSpec with Matchers with slogging.LazyLogging {
  "websocket publisher" should {

    val service = "testService"
    val method = "testMethod"
    val reqId = Random.nextLong()

    "send onComplete message when stream completed" in {

      val completePromise = Promise[Assertion]

      val task = for {
        topic ← async.topic[Task, WebSocketFrame](Ping())
        _ = topic
          .subscribe(10)
          .collect {
            case Binary(ab, _) ⇒
              WebsocketMessage.parseFrom(ab)
          }
          .map {
            case WebsocketMessage(s, m, rId, resp) if s == service && m == method && rId == reqId ⇒
              resp match {
                case Response.CompleteStatus(status) ⇒
                  completePromise.success(status.code.isOk shouldBe true)
                case _ ⇒ completePromise.success(true shouldBe false)
              }
            case m ⇒
              completePromise.success(true shouldBe false)
          }
          .compile
          .drain
          .runAsync
        publisher = new WebsocketPublishObserver(topic, service, method, reqId, true)
        _ = publisher.onComplete()
      } yield ()

      task.runAsync

      completePromise.future

    }

    "send onError message when stream errored" in {

      val errorPromise = Promise[Assertion]

      val task = for {
        topic ← async.topic[Task, WebSocketFrame](Ping())
        _ = topic
          .subscribe(10)
          .collect {
            case Binary(ab, _) ⇒
              WebsocketMessage.parseFrom(ab)
          }
          .map {
            case WebsocketMessage(s, m, rId, resp) if s == service && m == method && rId == reqId ⇒
              resp match {
                case Response.CompleteStatus(status) ⇒
                  errorPromise.success(status.code.isInternal shouldBe true)
                case _ ⇒ errorPromise.success(true shouldBe false)
              }
            case m ⇒
              errorPromise.success(true shouldBe false)
          }
          .compile
          .drain
          .runAsync
        publisher = new WebsocketPublishObserver(topic, service, method, reqId, true)
        _ = publisher.onError(new RuntimeException("msg"))
      } yield ()

      task.runAsync

      errorPromise.future

    }
  }
}
