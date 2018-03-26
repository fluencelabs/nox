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

import fluence.kad.grpc.{JSCodecs, KademliaNodeCodec}
import fluence.kad.grpc.facade._
import fluence.kad.protocol.{Contact, Key}
import cats.instances.try_._
import fluence.crypto.SignAlgo
import fluence.crypto.algorithm.Ecdsa

import scala.concurrent.Promise
import scala.scalajs.js.JSConverters._
import scala.scalajs.js
import scala.scalajs.js.annotation.JSExport
import scala.scalajs.js.typedarray.Uint8Array
import scala.util.Try

import scala.concurrent.ExecutionContext.Implicits.global

@JSExport
object Main {

  val algo: SignAlgo = Ecdsa.signAlgo
  import algo.checker

  @JSExport
  def someCoolMethod(): Unit = {
    println("Hello world!")

    var prKey: Promise[Key] = Promise[Key]()

    val grpc = Grpc
    val onMessage: Node ⇒ Unit = { out ⇒
      println("ON MESSAGE")
      println("NODE === " + out)
      val array = out.array.toArray
      println("idUINT === " + array)
      println("11111")
      val idBytes = JSCodecs.byteVectorUint8Array.encode(array.head)
      println("222222")
      val key = Key.fromBytes[Try](idBytes.get.toArray).get
      println("33333")
      println("KEY === " + key)
      prKey.success(key)
      val contact = Contact.readB64seed[Try](new String(array.tail.head.toJSArray.toArray.map(_.toByte)))
      println("CONTACT === " + contact)
      val codec = KademliaNodeCodec.codec[Try]
      println("codec === " + codec)
      val node = codec.decode(out)
      println("FULL NODE === " + node)
      println("FULL contact === " + node.get.contact)
    }

    val onEnd: InvokeOutput ⇒ Unit = { iout ⇒
      println("ONEND === " + iout)
    }

    val onHeaders: js.Any ⇒ Unit = { iout ⇒
      println("ONHEADERS === " + iout)
    }

    val descriptor = Kademlia.ping
    println(descriptor)

    val request = new PingRequest()
    println("After ping request")

    val options = InvokeRpcOptions[PingRequest, Node](
      request,
      "http://localhost:8080",
      "",
      onHeaders,
      onMessage,
      onEnd,
      debug = true
    )
    grpc.invoke[PingRequest, Node](descriptor, options)

    val pr2 = Promise[Key]()

    prKey.future.onComplete { somekey ⇒
      val descriptor2 = Kademlia.lookup
      val request2 = LookupRequest(new Uint8Array(somekey.get.value.toArray.toJSArray), 2)

      request2.setNumberofnodes(2)

      val onMessage2: NodesResponse ⇒ Unit = { nodesResp ⇒
        println("NODES RESP === " + nodesResp)

        val nodes = nodesResp.nodes()
        val codec = KademliaNodeCodec.codec[Try]
        val decNodes = nodes.map(n ⇒ codec.decode(n).get)
        println(decNodes.mkString("\n"))
        println("NODES SIZE === " + decNodes.length)

        pr2.success(decNodes.head.key)
      }

      val options2 = InvokeRpcOptions[LookupRequest, NodesResponse](
        request2,
        "http://localhost:8080",
        "",
        onHeaders,
        onMessage2,
        onEnd,
        debug = true
      )

      grpc.invoke[LookupRequest, NodesResponse](descriptor2, options2)
    }

    pr2.future.onComplete { somekey ⇒
      println("GOGOGO")
      val key = new Uint8Array(somekey.get.value.toArray.toJSArray)
      val key2 = new Uint8Array(prKey.future.value.get.get.value.toArray.toJSArray)
      println("GOGOGO1")
      val request3 = LookupAwayRequest(key, key2, 2)
      println("GOGOGO2")
      val descriptor3 = Kademlia.lookupAway
      println("GOGOGO3")
      val onMessage3: NodesResponse ⇒ Unit = { nodesResp ⇒
        println("NODES RESP === " + nodesResp)

        val nodes = nodesResp.nodes()
        val codec = KademliaNodeCodec.codec[Try]
        val decNodes = nodes.map(n ⇒ codec.decode(n).get)
        println(decNodes.mkString("\n"))
        println("NODES SIZE === " + decNodes.length)
      }

      val options2 = InvokeRpcOptions[LookupAwayRequest, NodesResponse](
        request3,
        "http://localhost:8080",
        "",
        onHeaders,
        onMessage3,
        onEnd,
        debug = true
      )

      grpc.invoke[LookupAwayRequest, NodesResponse](descriptor3, options2)
    }

  }

  def main(args: Array[String]): Unit = {
    someCoolMethod()
  }
}
