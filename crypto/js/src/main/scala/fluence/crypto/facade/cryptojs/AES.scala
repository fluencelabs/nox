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
import scala.scalajs.js.annotation.JSGlobal

@js.native
@JSGlobal
class AES extends js.Object {

  /**
    * @param msg Message to encrypt in JS WordArray.
    *            Could be created with CryptoJS.lib.WordArray.create(new Int8Array(arrayByte.toJSArray))
    * @param options { iv: iv, padding: CryptoJS.pad.Pkcs7, mode: CryptoJS.mode.CBC }
    * @return Encrypted message
    */
  def encrypt(msg: WordArray, key: Key, options: CryptOptions): js.Any = js.native

  def decrypt(encrypted: String, key: Key, options: CryptOptions): js.Any = js.native
}
