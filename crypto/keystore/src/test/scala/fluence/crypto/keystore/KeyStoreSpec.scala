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

  private val keyPair = KeyPair.fromBytes("pubKey".getBytes, "secKey".getBytes)
  private val jsonString = """{"keystore":{"secret":"c2VjS2V5","public":"cHViS2V5"}}"""

  "KeyStore.encodeKeyStorage" should {
    "transform KeyStore to json" in {
      val result = KeyStore.keyPairJsonStringCodec.direct.unsafe(keyPair)
      result shouldBe jsonString
    }
  }

  "KeyStore.decodeKeyStorage" should {
    "transform KeyStore to json" in {
      val result = KeyStore.keyPairJsonStringCodec.inverse.unsafe(jsonString)
      result shouldBe keyPair
    }
  }

  "KeyStore" should {
    "transform KeyStore to json and back" in {
      val result = KeyStore.keyPairJsonCodec.inverse.unsafe(KeyStore.keyPairJsonCodec.direct.unsafe(keyPair))
      result shouldBe keyPair
    }
  }

}
