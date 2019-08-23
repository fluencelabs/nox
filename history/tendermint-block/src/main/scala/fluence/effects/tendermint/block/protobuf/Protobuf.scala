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

package fluence.effects.tendermint.block.protobuf

import com.google.protobuf.{ByteString, CodedOutputStream}
import scalapb.{GeneratedMessage, GeneratedMessageCompanion, Message}
import scodec.bits.ByteVector

import scala.util.Try

/**
 * Collection of functions encoding primitives or generated protobuf structures into byte arrays
 */
private[block] object Protobuf {
  private def stringSize(s: String) = CodedOutputStream.computeStringSizeNoTag(s)
  private def int64Size(l: Long) = CodedOutputStream.computeInt64SizeNoTag(l)
  private def bytesSize(bs: ByteString) = CodedOutputStream.computeBytesSizeNoTag(bs)

  private def withOutput(size: => Int, f: CodedOutputStream => Unit): Array[Byte] = {
    val bytes = new Array[Byte](size)
    val out = CodedOutputStream.newInstance(bytes)
    f(out)
    out.flush()

    bytes
  }

  /**
   * Encodes String as protobuf's string
   *
   * @param s String value to be encoded
   * @return Byte array with encoded value
   */
  def encode(s: String): Array[Byte] = withOutput(stringSize(s), _.writeStringNoTag(s))

  /**
   * Encodes Long as protobuf's varint (int64, UVarInt64)
   *
   * @param l Long value to be encoded
   * @return Byte array with encoded value
   */
  def encode(l: Long): Array[Byte] = withOutput(int64Size(l), _.writeInt64NoTag(l))

  /**
   * Encodes byte vector in protobuf encoding, without field tag
   *
   * @param bv        ByteVector
   * @param skipEmpty If true, yields empty array on empty bs; yields [00] if skipEmpty = false
   * NOTE:
   *   In Tendermint's Go code, cdcEncode function is akin to skipEmpty = true,
   *   while MarshalBinaryBare is akin to skipEmpty = false
   *
   */
  def encode(bv: ByteVector, skipEmpty: Boolean): Array[Byte] = {
    if (bv.isEmpty && skipEmpty) {
      Array.empty
    } else {
      val bs = ByteString.copyFrom(bv.toArray)
      encode(bs, skipEmpty)
    }
  }

  /**
   * Encodes byte string in protobuf encoding, without field tag
   *
   * @param bs Byte string to encode
   * @param skipEmpty If true, yields empty array on empty bs; yields [00] if skipEmpty = false
   * NOTE:
   *   In Tendermint's Go code, cdcEncode function is akin to skipEmpty = true,
   *   while MarshalBinaryBare is akin to skipEmpty = false
   */
  def encode(bs: ByteString, skipEmpty: Boolean): Array[Byte] = {
    if (bs.isEmpty && skipEmpty) {
      Array.empty
    } else {
      withOutput(bytesSize(bs), _.writeBytesNoTag(bs))
    }
  }

  /**
   * Encodes optional protobuf message to either empty byte array or default protobuf encoding
   *
   * @param m Optional message to be encoded
   * @return Byte array, empty or containing encoded message
   */
  def encode[T <: GeneratedMessage](m: Option[T]): Array[Byte] = m.fold(Array.empty[Byte])(encode(_))

  /**
   * Encodes any protobuf message to byte array
   *
   * @param m Message to be encoded
   * @return Byte array with encoded message
   */
  def encode[T <: GeneratedMessage](m: T): Array[Byte] = m.toByteArray

  /**
   * Encodes a structure, prefixed with UVarInt encoding of the encoded structure's size
   *
   * In Go code: MarshalBinaryLengthPrefixed
   *
   * @param m Message to be encoded
   * @return Bytes of length-prefixed encoded message
   */
  def encodeLengthPrefixed[T <: GeneratedMessage](m: T): Array[Byte] = {
    val bytes = encode(m)
    val size = encode(bytes.length)
    size ++ bytes
  }

  /**
   * Encodes a list of optional protobuf structures:
   * 1. To a default protobuf structure serialization, if element is defined
   * 2. To an empty byte array, if element is None
   */
  def encode[T <: GeneratedMessage](repeated: List[Option[T]]): List[Array[Byte]] = {
    repeated.map(_.fold(Array.empty[Byte])(_.toByteArray))
  }

  def decode[T <: GeneratedMessage with Message[T]](
    bytes: Array[Byte]
  )(implicit companion: GeneratedMessageCompanion[T]): Either[Throwable, T] = {
    Try(companion.parseFrom(bytes)).toEither
  }
}
