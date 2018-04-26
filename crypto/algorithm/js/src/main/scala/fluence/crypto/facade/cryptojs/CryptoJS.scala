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
import scala.scalajs.js.annotation.JSImport

@js.native
@JSImport("crypto-js", JSImport.Namespace)
object CryptoJS extends js.Object {

  def pad: Paddings = js.native
  def mode: Modes = js.native
  def AES: AES = js.native

  /**
   * https://en.wikipedia.org/wiki/PBKDF2
   * @return Salted and hashed key
   */
  def PBKDF2(pass: String, salt: String, options: KeyOptions): Key = js.native

  def lib: Lib = js.native

  def enc: Enc = js.native

  def algo: Algos = js.native
}
