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

import fluence.crypto.facade.ecdsa.{SHA1, SHA256}
import org.scalatest.{Matchers, WordSpec}

import scala.scalajs.js.JSConverters._

class JSHashSpec extends WordSpec with Matchers {
  "js hasher" should {
    //test values get from third-party hash services
    "work with sha256" in {
      val str = "sha256Tester"
      val sha256TesterHex = "513c17f8cf6ba96ce412cc2ae82f68821e9a2c6ae7a2fb1f5e46d08c387c8e65"

      val hasher = new SHA256()
      hasher.update(str.getBytes().toJSArray)
      val hex = hasher.digest("hex")
      hex shouldBe sha256TesterHex
    }

    "work with sha1" in {
      val str = "sha1Tester"
      val sha1TesterHex = "879db20eabcecea7d4736a8bae5bc64564b76b2f"

      val hasher = new SHA1()
      hasher.update(str.getBytes().toJSArray)
      val hex = hasher.digest("hex")
      hex shouldBe sha1TesterHex
    }
  }
}
