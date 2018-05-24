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

    val seedContact = Contact.readB64seed.unsafe(
      "eyJwayI6IkF3RDNJWlJsYlFidkFISlRfRzYxRnZkT0xtVHRjdDU1NlJjZDBmeGxOa3huIiwicHYiOjB9.eyJhIjoiZGllbXVzdC1HUzQzVlItN1JFIiwiZ3AiOjExMDIxLCJnaCI6IjAwMDAwMDAwMDAwMDAwMDAwMDAwIiwid3AiOjgwOTF9.MEUCIQCfncSwcDNSSET29HJqeenxGbYCpB3RcC_kgcD3CPSMQwIgCSdT_JFHHXPhpEpkVqovbNBzteM96TN1Osh045SDiY0="
    )

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
      dataset ← cl.createNewContract(newkey, 1, keyCrypt, valueCrypt).toIO
      _ = println("222222222")
      _ ← {
        (for {
          b ← dataset.get("1234")
          _ = println("GETTED B === " + b)
          _ ← dataset.put("1", "23")
          res ← dataset.put("1235", "123")
          _ = println("RESULT === " + res)
          _ = println("444444444444")
          a ← dataset.get("1235")
          _ = println("GETTED === " + a)
          _ = println("55555555555")
          _ ← dataset.get("1236")
          _ = println("666666666666")
        } yield ()).toIO
      }

    } yield {
      println("HURAAAH!!")
    }

    r.attempt.unsafeToFuture()
  }

  def main(args: Array[String]): Unit = {
    println("start main")
    logic()
  }
}
