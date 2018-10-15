/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package fluence.statemachine.util
import scodec.bits.{Bases, ByteVector}

object HexCodec {

  /**
   * Converts binary data to uppercase hex representation.
   *
   * @param bytes binary data
   */
  def binaryToHex(bytes: ByteVector): String = bytes.toHex(Bases.Alphabets.HexUppercase)

  /**
   * Converts binary data to uppercase hex representation.
   *
   * @param bytes binary data
   */
  def binaryToHex(bytes: Array[Byte]): String = binaryToHex(ByteVector(bytes))

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
  def hexToString(hex: String): Either[String, String] =
    ByteVector.fromHexDescriptive(hex, Bases.Alphabets.HexUppercase).map(x => new String(x.toArray))

  /**
   * Decodes a string from hex representation to array of bytes.
   *
   * @param hex string in hex representation
   */
  def hexToArray(hex: String): Either[String, Array[Byte]] =
    ByteVector.fromHexDescriptive(hex, Bases.Alphabets.HexUppercase).map(_.toArray)
}
