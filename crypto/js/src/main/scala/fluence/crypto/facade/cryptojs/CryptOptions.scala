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

package fluence.crypto.facade.cryptojs

import scala.scalajs.js

@js.native
trait CryptOptions extends js.Object {
  val iv: Option[js.Any]
  val padding: Pad
  val mode: Mode
}

object CryptOptions {
  def apply(iv: Option[WordArray], padding: Pad, mode: Mode): CryptOptions = {
    iv match {
      case Some(i) ⇒
        js.Dynamic.literal(iv = i, padding = padding, mode = mode).asInstanceOf[CryptOptions]
      case None ⇒
        //if IV is empty, there will be an error in JS lib
        js.Dynamic.literal(iv = CryptoJS.lib.WordArray.random(0), padding = padding, mode = mode).asInstanceOf[CryptOptions]
    }

  }
}
