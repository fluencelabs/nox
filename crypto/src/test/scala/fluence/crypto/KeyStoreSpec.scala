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

import fluence.crypto.keypair.KeyPair
import org.scalatest.{ Matchers, WordSpec }
import scodec.bits.{ Bases, ByteVector }

class KeyStoreSpec extends WordSpec with Matchers {

  private val alphabet = Bases.Alphabets.Base64Url

  private val keyStore = KeyStore(KeyPair.fromBytes("pubKey".getBytes, "secKey".getBytes))
  private val jsonString = """{"keystore":{"secret":"c2VjS2V5","public":"cHViS2V5"}}"""

  "KeyStore.encodeKeyStorage" should {
    "transform KeyStore to json" in {
      val result = KeyStore.encodeKeyStorage(keyStore)
      result.noSpaces shouldBe jsonString
    }
  }

  "KeyStore.decodeKeyStorage" should {
    "transform KeyStore to json" in {
      import io.circe.parser._
      val result = KeyStore.decodeKeyStorage.decodeJson(parse(jsonString).right.get)
      result.right.get.get shouldBe keyStore
    }
  }

  "KeyStore" should {
    "transform KeyStore to json and back" in {
      val result = KeyStore.decodeKeyStorage.decodeJson(KeyStore.encodeKeyStorage(keyStore))
      result.right.get.get shouldBe keyStore
    }
  }

  "fromBase64" should {
    "throw an exception" when {
      "invalid base64 format" in {
        val invalidBase64 = "!@#$%"

        val e = KeyStore.fromBase64(invalidBase64).value.left.get
        e should have message "'" + invalidBase64 + "' is not a valid base64."
      }
      "invalid decoded json" in {
        val invalidJson = ByteVector("""{"keystore":{"public":"cHViS2V5"}}""".getBytes).toBase64(alphabet)

        KeyStore.fromBase64(invalidJson).value.isLeft shouldBe true

      }
    }

    "fetch KeyStore from valid base64" in {
      val invalidJson = ByteVector(jsonString.getBytes).toBase64(alphabet)
      val result = KeyStore.fromBase64(invalidJson).value.right.get
      result shouldBe keyStore
    }
  }

}
