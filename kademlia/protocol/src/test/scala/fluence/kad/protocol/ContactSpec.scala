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
import fluence.crypto.algorithm.Ecdsa
import org.scalatest.{ Matchers, WordSpec }

class ContactSpec extends WordSpec with Matchers {

  "Contact" should {
    "serialize and deserialize in Id" in {

      val algo = Ecdsa.signAlgo
      import algo.checker

      val Right(kp) = algo.generateKeyPair[Id]().value

      val c = Contact.buildOwn[Id](
        "127.0.0.1",
        8080,
        10l,
        "hash",
        algo.signer(kp)
      ).value.right.get

      val seed = c.b64seed

      Contact.readB64seed[Id](seed).value.isRight shouldBe true
      Contact.readB64seed[Id](seed).value shouldBe Right(c)
    }
  }

}
