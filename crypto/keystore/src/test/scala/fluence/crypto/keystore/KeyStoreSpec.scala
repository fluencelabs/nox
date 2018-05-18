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

package fluence.crypto.keystore

import fluence.crypto.KeyPair
import org.scalatest.{Matchers, WordSpec}

class KeyStoreSpec extends WordSpec with Matchers {

  private val keyStore = KeyStore(KeyPair.fromBytes("pubKey".getBytes, "secKey".getBytes))
  private val jsonString = """{"keystore":{"secret":"c2VjS2V5","public":"cHViS2V5"}}"""

  "KeyStore.encodeKeyStorage" should {
    "transform KeyStore to json" in {
      val result = KeyStore.keyStoreJsonStringCodec.direct.unsafe(keyStore)
      result shouldBe jsonString
    }
  }

  "KeyStore.decodeKeyStorage" should {
    "transform KeyStore to json" in {
      val result = KeyStore.keyStoreJsonStringCodec.inverse.unsafe(jsonString)
      result shouldBe keyStore
    }
  }

  "KeyStore" should {
    "transform KeyStore to json and back" in {
      val result = KeyStore.keyStoreJsonCodec.inverse.unsafe(KeyStore.keyStoreJsonCodec.direct.unsafe(keyStore))
      result shouldBe keyStore
    }
  }

}
