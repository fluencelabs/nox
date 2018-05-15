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

import fluence.codec.PureCodec
import fluence.crypto.ecdsa.Ecdsa
import fluence.crypto.signature.SignAlgo
import fluence.kad.grpc.client.KademliaWebsocketClient
import fluence.kad.protocol.Key
import fluence.proxy.grpc.WebsocketMessage
import fluence.transport.websocket.{Websocket, WebsocketPipe, WebsocketT}
import monix.execution.Scheduler.Implicits.global
import slogging.{LogLevel, LoggerConfig, PrintLoggerFactory}

import scala.concurrent.duration._
import scala.scalajs.js.annotation.{JSExport, JSExportTopLevel}

/**
 *
 * This is class for tests only, will be deleted after implementation of browser client.
 *
 */
@JSExportTopLevel("Main")
object Main extends slogging.LazyLogging {

  LoggerConfig.factory = PrintLoggerFactory()
  LoggerConfig.level = LogLevel.DEBUG

  import fluence.kad.KademliaNodeCodec._

  @JSExport
  def logic(): Unit = {

    val algo: SignAlgo = Ecdsa.signAlgo
    import algo.checker

    val host = "ws://127.0.0.1:8090/ws"

    val builder: String ⇒ WebsocketT = str ⇒ Websocket(str)
    val wsRawClient = WebsocketPipe.binaryClient(host, builder, 1, 10.millis)

    val websocketMessageCodec =
      PureCodec.build[WebsocketMessage, Array[Byte]](
        (a: WebsocketMessage) ⇒ a.toByteString.toByteArray,
        (ab: Array[Byte]) ⇒ WebsocketMessage.parseFrom(ab)
      )

    val ws =
      wsRawClient.xmap[WebsocketMessage, WebsocketMessage](websocketMessageCodec.direct, websocketMessageCodec.inverse)
    val client = new KademliaWebsocketClient(ws)

    val keyP = algo.generateKeyPair.unsafe(None)
    println("KEYP === " + keyP)
    val key = Key.fromPublicKey(keyP.publicKey).value.toOption.get
    println("KEY === " + key)
    println("PINGING")
    val io = for {
      node ← client.ping()
      _ = println("Ping node response: " + node)
      _ = logger.info("Ping node response: " + node)
      listOfNodes ← client.lookup(key, 2)
      _ = println("Lookup nodes response: " + listOfNodes.mkString("\n"))
      _ = logger.info("Lookup nodes response: " + listOfNodes.mkString("\n"))
      key2 = listOfNodes.head.key
      listOfNodes2 ← client.lookupAway(key2, key, 2)
    } yield {
      logger.info("Lookup away nodes response: " + listOfNodes2.mkString("\n", "\n", "\n"))
      println("Lookup away nodes response: " + listOfNodes2.mkString("\n", "\n", "\n"))
    }

    io.attempt.map { e ⇒
      println("EITHER E === " + e)
      e
    }.unsafeRunAsync(res ⇒ logger.info("Result: " + res))
  }

  def main(args: Array[String]): Unit = {
//    logic()
  }
}
