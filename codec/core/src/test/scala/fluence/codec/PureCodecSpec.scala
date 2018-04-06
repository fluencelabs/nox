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

package fluence.codec

import cats.instances.try_._
import org.scalatest.prop.Checkers
import org.scalatest.{Matchers, WordSpec}

import scala.util.Try

class PureCodecSpec extends WordSpec with Matchers with Checkers {

  "PureCodec" should {
    "summon implicit identity" in {
      val intId = implicitly[PureCodec[Int, Int]]
      check { (i: Int) ⇒
        intId.direct.runF[Try](i).get == i &&
        intId.inverse.runF[Try](i).get == i
      }

    }
  }
}
