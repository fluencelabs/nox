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

import cats.instances.try_._
import fluence.crypto.SignAlgo
import fluence.crypto.algorithm.Ecdsa
import fluence.kad.grpc.facade._
import fluence.kad.grpc.{KademliaGrpcService, KademliaNodeCodec}

import scala.concurrent.ExecutionContext.Implicits.global
import scala.scalajs.js.JSConverters._
import scala.scalajs.js.annotation.JSExport
import scala.scalajs.js.typedarray.Uint8Array
import scala.util.Try

@JSExport
object Main {

  val algo: SignAlgo = Ecdsa.signAlgo
  import algo.checker

  val host = "http://localhost:8080"

  val grpcService = KademliaGrpcService(host, false)

  @JSExport
  def someCoolMethod(): Unit = {
    println("Hello world!")
    for {
      jsNode ← grpcService.ping(PingRequest())
      codec = KademliaNodeCodec.codec[Try]
      node = codec.decode(jsNode).get
      _ = println("NODE === " + node)
      key = new Uint8Array(node.key.id.toJSArray)
      _ = println("JS KEY === " + key)
      listOfNodes ← grpcService.lookup(LookupRequest(key, 2))
      decNodes = listOfNodes.nodes().map(n ⇒ codec.decode(n).get)
      _ = println("LOOKUP NODES === " + decNodes.mkString("\n"))
      key2 = new Uint8Array(decNodes.head.key.id.toJSArray)
      listOfNodes2 ← grpcService.lookupAway(LookupAwayRequest(key2, key, 2))
      decNodes2 = listOfNodes2.nodes().map(n ⇒ codec.decode(n).get)
    } yield {
      println("LOOKUP AWAY NODES === " + decNodes2.mkString("\n"))
    }
  }

  def main(args: Array[String]): Unit = {
    someCoolMethod()
  }
}
