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

package fluence.client.grpc

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
import cats.{Applicative, Id}
import fluence.client.core.FluenceClient
import fluence.crypto.aes.{AesConfig, AesCrypt}
import fluence.crypto.ecdsa.Ecdsa
import fluence.crypto.hash.JsCryptoHasher
import fluence.crypto.signature.SignAlgo
import fluence.crypto.{Crypto, KeyPair}
import fluence.kad.KademliaConf
import fluence.kad.protocol.Contact
import monix.eval.Task
import slogging.{LogLevel, LoggerConfig, PrintLoggerFactory}

import monix.execution.Scheduler.Implicits.global

import scala.concurrent.duration._
import scala.language.higherKinds
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

    val seedContact = Contact
      .readB64seed[Id](
        "eyJwayI6IkFfZmhNQXA5TTluc2czczkzREZNV3ZBMnd6NHlqTnVvZy1DWGVIWU4yeUhjIiwicHYiOjB9.eyJhIjoiZGllbXVzdC1HUzQzVlItN1JFIiwiZ3AiOjExMDIyLCJnaCI6IjAwMDAwMDAwMDAwMDAwMDAwMDAwIiwid3AiOjgwOTJ9.MEUCIQDuTTVy4LoaKd2rHTZmPxuFmNBO64v1yJ5bMz6IHpaz5AIgCqbXdbyBxPaIs0srjhGV9nQodmprTjDamYh8MFVby9U="
      )
      .value
      .right
      .get

    val kadConfig = KademliaConf(3, 3, 1, 5.seconds)

    val client = ClientWebsocketServices.build[Task]

    val clIO = FluenceClient.build(Seq(seedContact), algo, hasher, kadConfig, client)

    def cryptoMethods(
      secretKey: KeyPair.Secret
    ): (Crypto.Cipher[String], Crypto.Cipher[String]) = {
      val aesConfig = AesConfig(
        50
      )
      (
        AesCrypt.forString(secretKey.value, withIV = false, aesConfig),
        AesCrypt.forString(secretKey.value, withIV = true, aesConfig)
      )
    }

    val r = for {
      cl ← clIO
      newkey ← algo.generateKeyPair.runF[IO](None)
      crypts = cryptoMethods(newkey.secretKey)
      (keyCrypt, valueCrypt) = crypts
      _ = println("111111111")
      dataset ← cl.createNewContract(newkey, 2, keyCrypt, valueCrypt).toIO
      _ = println("222222222")
      _ ← dataset.get("1234").toIO
      _ = println("232323232323")
      _ ← dataset.put("1", "").toIO
      _ ← dataset.put("1235", "123").toIO
      _ = println("3333333333")
      _ ← dataset.get("1236").toIO
    } yield {}

    r.attempt.unsafeToFuture()
  }

  def main(args: Array[String]): Unit = {
    println("start main")
    logic()
  }
}
