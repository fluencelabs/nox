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

package fluence.statemachine.util

object HexCodec {

  /**
   * Converts binary data to uppercase hex representation.
   *
   * @param bytes binary data
   */
  def binaryToHex(bytes: Array[Byte]): String = bytes.map("%02X".format(_)).mkString

  /**
   * Encodes text data to uppercase hex representation corresponding to UTF-8 encoding of the data.
   *
   * @param data text data
   */
  def stringToHex(data: String): String = binaryToHex(data.getBytes("UTF-8"))

  /**
   * Decodes text data from hex representation corresponding to UTF-8 encoding of the data.
   *
   * @param hex hex representation
   */
  def hexToString(hex: String): String = hex.sliding(2, 2).toArray.map(Integer.parseInt(_, 16).toChar).mkString
}
