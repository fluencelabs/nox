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

package fluence.transport.websocket

import fluence.codec
import fluence.codec.PureCodec
import monix.eval.Task
import monix.reactive.Observable
import org.scalatest.{AsyncWordSpec, Matchers}

import scala.concurrent.duration._
import scala.scalajs.js.Date

class ConnectionPoolSpec extends AsyncWordSpec with Matchers {

  implicit override def executionContext = monix.execution.Scheduler.Implicits.global

  "connection pool" should {
    "clean up connections by timeout" in {
      val timeout = {
        val date = new Date(0)
        date.setMilliseconds(200)
        date
      }

      case class Wrapper(arr: Array[Byte])

      implicit val wrapperCodec: codec.PureCodec[Wrapper, Array[Byte]] =
        PureCodec.build[Wrapper, Array[Byte]](
          (m: Wrapper) ⇒ m.arr: Array[Byte],
          (arr: Array[Byte]) ⇒ Wrapper(arr)
        )

      val pool = ConnectionPool[Wrapper](timeout, 100.millis, builder = WebsocketEcho.builder)

      val client1 = pool.getOrCreateConnection("1")
      val client2 = pool.getOrCreateConnection("2")
      val client3 = pool.getOrCreateConnection("3")

      val subscription2 =
        Observable.timerRepeated(0.seconds, 100.millis, Wrapper(Array[Byte](1, 2, 3))).subscribe(client2.input)

      println("111111111")

      (for {
        _ ← Task.sleep(500.millis)
      } yield {
        println("22222222")
        val connections = pool.getConnections

        subscription2.cancel()

        connections.size shouldBe 1

        connections.keys.toSeq should contain theSameElementsAs Seq("2")
      }).runAsync
    }
  }
}
