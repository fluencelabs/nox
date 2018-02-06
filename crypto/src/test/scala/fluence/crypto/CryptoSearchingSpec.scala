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

package fluence.crypto

import fluence.crypto.cipher.NoOpCrypt
import org.scalatest.{ Matchers, WordSpec }

import scala.collection.Searching.{ Found, InsertionPoint }

class CryptoSearchingSpec extends WordSpec with Matchers {

  "search" should {
    "correct search plainText key in encrypted data" in {

      val crypt: NoOpCrypt[String] = NoOpCrypt.forString

      val plainTextElements = Array("A", "B", "C", "D", "E")
      val encryptedElements = plainTextElements.map(crypt.encrypt)

      import fluence.crypto.cipher.CryptoSearching._
      implicit val decryptFn: (Array[Byte]) ⇒ String = crypt.decrypt

      encryptedElements.binarySearch("B") shouldBe Found(1)
      encryptedElements.binarySearch("D") shouldBe Found(3)
      encryptedElements.binarySearch("E") shouldBe Found(4)

      encryptedElements.binarySearch("0") shouldBe InsertionPoint(0)
      encryptedElements.binarySearch("BB") shouldBe InsertionPoint(2)
      encryptedElements.binarySearch("ZZ") shouldBe InsertionPoint(5)

    }
  }

}
