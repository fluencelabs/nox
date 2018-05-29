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

package fluence.client

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

import cats.effect.IO
import fluence.client.core.FluenceClient
import fluence.client.grpc.ClientWebsocketServices
import fluence.codec
import fluence.codec.PureCodec
import fluence.crypto.aes.{AesConfig, AesCrypt}
import fluence.crypto.ecdsa.Ecdsa
import fluence.crypto.hash.JsCryptoHasher
import fluence.crypto.signature.SignAlgo
import fluence.crypto.{Crypto, KeyPair}
import fluence.kad.KademliaConf
import fluence.kad.protocol.Contact
import fluence.proxy.grpc.WebsocketMessage
import fluence.transport.websocket.{ConnectionPool, Websocket}
import monix.eval.Task
import monix.execution.Scheduler.Implicits.global
import slogging.{LogLevel, LoggerConfig, PrintLoggerFactory}

import scala.concurrent.duration._
import scala.language.higherKinds
import scala.scalajs.js.Date
import scala.scalajs.js.annotation.{JSExport, JSExportTopLevel}

/**
 *
 * This is class for tests only, will be deleted after implementation of browser client.
 *
 */
@JSExportTopLevel("MainD")
object Main extends slogging.LazyLogging {

  LoggerConfig.factory = PrintLoggerFactory()
  LoggerConfig.level = LogLevel.DEBUG

  @JSExport
  def logic(): Unit = {

    val algo: SignAlgo = Ecdsa.signAlgo
    import algo.checker

    val hasher: Crypto.Hasher[Array[Byte], Array[Byte]] = JsCryptoHasher.Sha256

    val seedContact = Contact.readB64seed.unsafe(
      "eyJwayI6IkE1dXlwajBkcXBZdDYtcWNvMmhMME14Y2Flbm4xUHF2X1FmTXNrMG1uaFpDIiwicHYiOjB9.eyJhIjoiMTI3LjAuMC4xIiwiZ3AiOjExMDIyLCJnaCI6IjAwMDAwMDAwMDAwMDAwMDAwMDAwIiwid3AiOjgwOTJ9.MEUCIE94PSeplPfqkoDGU22ckOxQv2gpb5TN2E2MbSQIvTr7AiEAyLrGLC4RnsqlBuAo-AmegXeibYpjTtjteZHRAZrHdf8="
    )

    val kadConfig = KademliaConf(3, 3, 1, 5.seconds)

    val timeout = {
      val date = new Date(0)
      date.setSeconds(3)
      date
    }

    implicit val websocketMessageCodec: codec.PureCodec[WebsocketMessage, Array[Byte]] =
      PureCodec.build[WebsocketMessage, Array[Byte]](
        (m: WebsocketMessage) ⇒ m.toByteArray,
        (arr: Array[Byte]) ⇒ WebsocketMessage.parseFrom(arr)
      )

    val connectionPool = ConnectionPool[WebsocketMessage](timeout, 1.second, builder = Websocket.builder)
    val clientWebsocketServices = new ClientWebsocketServices(connectionPool)

    val client = clientWebsocketServices.build[Task]

    val clIO = FluenceClient.build(Seq(seedContact), algo, hasher, kadConfig, client andThen (_.get))

    def cryptoMethods(
      secretKey: KeyPair.Secret
    ): (Crypto.Cipher[String], Crypto.Cipher[String]) = {
      val aesConfig = AesConfig(
        50
      )
      (
        AesCrypt.forString(secretKey.value, withIV = false, aesConfig),
        AesCrypt.forString(secretKey.value, withIV = false, aesConfig)
      )
    }

    val r = for {
      cl ← clIO
      newkey ← algo.generateKeyPair.runF[IO](None)
      crypts = cryptoMethods(newkey.secretKey)
      (keyCrypt, valueCrypt) = crypts
      dataset ← cl.createNewContract(newkey, 2, keyCrypt, valueCrypt).toIO
      _ ← {
        (for {
          a ← dataset.get("1234")
          _ = println("a == None: " + a.isEmpty)
          _ ← dataset.put("1", "23")
          res ← dataset.put("1235", "123")
          _ = println("res == None: " + res.isEmpty)
          b ← dataset.get("1235")
          _ = println("b == 123: " + b.contains("123"))
          c ← dataset.get("1236")
          _ = println("c == None: " + c.isEmpty)
        } yield ()).toIO
      }
      _ ← IO.sleep(6.seconds)
      _ ← {
        (for {
          a ← dataset.get("1234")
          _ = println("a == None: " + a.isEmpty)
          _ ← dataset.put("1", "23")
          res ← dataset.put("1235", "123")
          _ = println("res == None: " + res.isEmpty)
          b ← dataset.get("1235")
          _ = println("b == 123: " + b.contains("123"))
          c ← dataset.get("1236")
          _ = println("c == None: " + c.isEmpty)
        } yield {}).toIO
      }
    } yield {
      println("finished")
    }

    r.attempt.unsafeToFuture()
  }

  def main(args: Array[String]): Unit = {
    println("start main")
//    logic()
  }

  logic()
}
