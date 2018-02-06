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

class NoOpCryptSpec extends WordSpec with Matchers {

  "NoOpCrypt" should {
    "convert a string to bytes back and forth without any cryptography" in {

      val noOpCrypt = NoOpCrypt.forString

      val emptyString = ""
      noOpCrypt.decrypt(noOpCrypt.encrypt(emptyString)) shouldBe emptyString
      val nonEmptyString = "some text here"
      noOpCrypt.decrypt(noOpCrypt.encrypt(nonEmptyString)) shouldBe nonEmptyString
      val byteArray = Array(1.toByte, 23.toByte, 45.toByte)
      noOpCrypt.encrypt(noOpCrypt.decrypt(byteArray)) shouldBe byteArray
    }

    "convert a long to bytes back and forth without any cryptography" in {

      val noOpCrypt = NoOpCrypt.forLong

      noOpCrypt.decrypt(noOpCrypt.encrypt(0L)) shouldBe 0L
      noOpCrypt.decrypt(noOpCrypt.encrypt(1234567890123456789L)) shouldBe 1234567890123456789L
    }
  }
}
