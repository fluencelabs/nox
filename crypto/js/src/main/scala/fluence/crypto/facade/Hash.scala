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

package fluence.crypto.facade

import scala.scalajs.js
import scala.scalajs.js.annotation.JSImport

//TODO hide enc argument in methods, make it `hex` by default
/**
 * https://github.com/indutny/hash.js - part of elliptic library
 */
@js.native
@JSImport("hash.js", "sha256")
class SHA256() extends js.Object {
  def update(msg: js.Array[Byte]): Unit = js.native
  def digest(enc: String): String = js.native
}

@js.native
@JSImport("hash.js", "sha1")
class SHA1() extends js.Object {
  def update(msg: js.Array[Byte]): Unit = js.native
  def digest(enc: String): String = js.native
}
