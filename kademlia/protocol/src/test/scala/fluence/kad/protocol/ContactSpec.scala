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

package fluence.kad.protocol

import cats._
import fluence.crypto.ecdsa.Ecdsa
import fluence.kad.protocol.Contact.JwtData
import io.circe.Decoder
import io.circe.parser._
import org.scalatest.{Matchers, WordSpec}
import scodec.bits.Bases

class ContactSpec extends WordSpec with Matchers {

  "Contact" should {
    "serialize and deserialize in Id" in {

      val algo = Ecdsa.signAlgo
      import algo.checker

      val kp = algo.generateKeyPair.unsafe(None)

      val c = Contact
        .buildOwn[Id](
          "127.0.0.1",
          8080,
          Some(8090),
          10l,
          "hash",
          algo.signer(kp)
        )
        .value
        .right
        .get

      val seed = c.b64seed

      Contact.readB64seed[Id](seed).value.isRight shouldBe true
      Contact.readB64seed[Id](seed).value shouldBe Right(c)
    }

    "serialize and deserialize without websocket port" in {
      val algo = Ecdsa.signAlgo
      import algo.checker

      val kp = algo.generateKeyPair.unsafe(None)

      val c = Contact
        .buildOwn[Id](
          "127.0.0.1",
          8080,
          None,
          10l,
          "hash",
          algo.signer(kp)
        )
        .value
        .right
        .get

      val seed = c.b64seed

      Contact.readB64seed[Id](seed).value.isRight shouldBe true
      Contact.readB64seed[Id](seed).value shouldBe Right(c)
    }

    "serialize and deserialize JwtData with null in json" in {

      import Contact.JwtImplicits._

      val json = """{
                   |    "a" : "127.0.0.1",
                   |    "gp" : 8080,
                   |    "gh" : "hash",
                   |    "wp" : null
                   |}""".stripMargin

      val alphabet = Bases.Alphabets.Base64Url

      val data = Decoder[JwtData].decodeJson(parse(json).toOption.get).right.get

      data shouldBe JwtData("127.0.0.1", 8080, None, "hash")
    }
  }

}
