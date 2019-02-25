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

package fluence.ethclient.helpers
import java.util.Base64

import org.web3j.abi.datatypes.generated._
import org.web3j.abi.datatypes.{DynamicArray, Type}
import scodec.bits.{Bases, ByteVector}

object Web3jConverters {

  /**
   * Converts string to web3j's Bytes32.
   *
   * @param s string
   */
  def stringToBytes32(s: String): Bytes32 = {
    val byteValue = s.getBytes()
    val byteValueLen32 = new Array[Byte](32)
    System.arraycopy(byteValue, 0, byteValueLen32, 0, byteValue.length)
    new Bytes32(byteValueLen32)
  }

  /**
   * Converts byte vector to hex string trimming leading zeros.
   *
   * @param b byte vector
   */
  def binaryToHexTrimZeros(b: ByteVector): String = b.dropWhile(_ == 0).toHex

  /**
   * Converts base64 string to web3j's Bytes32.
   *
   * @param base64 base64 string
   */
  def base64ToBytes32(base64: String): Bytes32 = new Bytes32(Base64.getDecoder.decode(base64))

  /**
   * Converts web3j's Bytes32 to base64 string
   *
   * @param bytes32 bytes32 value
   */
  def bytes32ToBase64(bytes32: Bytes32): String = Base64.getEncoder.encodeToString(bytes32.getValue)

  /**
   * Converts bytes of web3j's Bytes32 to ByteVector
   *
   * @param bytes32 bytes32 value
   */
  def bytes32ToBinary(bytes32: Bytes32): ByteVector = ByteVector(bytes32.getValue)

  /**
   * Converts hex string to byte array.
   * TODO: add checks, now it's unsafe.
   *
   * @param hex hex string
   */
  def hexToBinary(hex: String): Array[Byte] =
    ByteVector.fromHex(hex, Bases.Alphabets.HexUppercase).map(_.toArray).getOrElse(new Array[Byte](hex.length / 2))

  /**
   * Converts hex string to Bytes32.
   *
   * @param hex hex string
   */
  def hexToBytes32(hex: String): Either[Throwable, Bytes32] = {
    val binary = hexToBinary(hex)
    if (binary.length == 32) {
      Right(new Bytes32(hexToBinary(hex)))
    } else {
      Left(new Exception("Incorrect bytes length for 'hex': must be 32 bytes"))
    }
  }

  /**
   * Encodes worker address information to web3j's Bytes24.
   *
   * @param ip worker host IP
   * @param nodeAddressHex Tendermint p2p public key
   */
  def nodeAddressToBytes24(ip: String, nodeAddressHex: String): Bytes24 = {
    val buffer = new Array[Byte](24)

    Array.copy(hexToBinary(nodeAddressHex), 0, buffer, 0, 20)
    Array.copy(ip.split('.').map(_.toInt.toByte), 0, buffer, 20, 4)

    new Bytes24(buffer)
  }

  def listToDynamicArray[T <: Type[_]](list: List[T]): DynamicArray[T] =
    if (list.isEmpty) {
      DynamicArray.empty("bytes32[]").asInstanceOf[DynamicArray[T]]
    } else {
      new DynamicArray[T](list: _*)
    }

}
