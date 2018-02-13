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
import scala.scalajs.js.annotation._

//TODO hide enc argument in methods, make it `hex` by default
/**
 * https://github.com/indutny/elliptic - fast elliptic-curve cryptography in a plain javascript implementation
 */
@js.native
@JSImport("elliptic", "ec")
class EC(curve: String) extends js.Object {
  def genKeyPair(options: Option[js.Dynamic] = None): KeyPair = js.native
  def keyPair(options: js.Dynamic): KeyPair = js.native
  def keyFromPublic(pub: String, enc: String): KeyPair = js.native
  def keyFromPrivate(priv: String, enc: String): KeyPair = js.native
}

@js.native
@JSImport("elliptic", "ec")
class KeyPair(ec: EC, options: js.Dynamic) extends js.Object {
  def verify(msg: js.Array[Byte], signature: String): Boolean = js.native
  def sign(msg: js.Array[Byte]): Signature = js.native

  def getPublic(compact: Boolean, enc: String): String = js.native
  def getPrivate(enc: String): String = js.native
  val priv: js.Any = js.native
  val pub: js.Any = js.native
}

@js.native
@JSImport("elliptic", "ec")
class Signature(der: String, enc: String = "hex") extends js.Object {
  def toDER(enc: String): String = js.native
}
