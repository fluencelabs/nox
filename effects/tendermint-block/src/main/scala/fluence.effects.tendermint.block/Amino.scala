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

package fluence.effects.tendermint.block

import com.google.protobuf.{ByteString, CodedOutputStream}
import scalapb.GeneratedMessage
import scodec.bits.ByteVector

object Amino {
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

  def encode(s: String): Array[Byte] = withOutput(stringSize(s), _.writeStringNoTag(s))
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
   * @param bs ByteString
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

  def encode[T <: GeneratedMessage](m: Option[T]): Array[Byte] = m.fold(Array.empty[Byte])(encode(_))
  def encode[T <: GeneratedMessage](m: T): Array[Byte] = m.toByteArray

  // go: MarshalBinaryLengthPrefixed
  // Encodes a structure, prefixed with UVarInt encoding of the encoded structure's size
  // It's all happening for Go reasons
  def encodeLengthPrefixed[T <: GeneratedMessage](m: T): Array[Byte] = {
    val bytes = encode(m)
    val size = encode(bytes.length)
    println(s"encodeLengthPrefixed size: ${bytes.length} -> ${ByteVector(size).toHex}")
    size ++ bytes
  }

  def encodeJava[T <: com.google.protobuf.GeneratedMessageV3](m: T): Array[Byte] = m.toByteArray

  private val NoneBytes = Array[Byte](0)

  def encode[T <: GeneratedMessage](repeated: List[Option[T]]): Array[Byte] = {
    import CodedOutputStream.{computeUInt32SizeNoTag => sizeBytes}
    val bytes = repeated.map(_.map(_.toByteArray))

    // 1 for tag before each element, sizeBytes to encode size of each element, and element itself
    val size = bytes.map(1 + _.fold(NoneBytes.length)(b => sizeBytes(b.length) + b.length)).sum

    withOutput(
      size,
      out => {
        bytes.foreach {
          maybeBytes =>
            def write(out: CodedOutputStream): Unit = {
              out.writeTag(2, 2) // becomes 0x12
              maybeBytes
                .foreach(bs => out.writeUInt32NoTag(bs.length)) // Encode length for existing values; skip for None
              val array = maybeBytes.getOrElse(NoneBytes)
              out.write(array, 0, array.length)
//              println(s"Bytes written ${out.getTotalBytesWritten}")
            }

            val len = maybeBytes.fold(NoneBytes.length)(_.length)
            val szBytes = maybeBytes.fold(0)(_ => sizeBytes(len))
            val outLen = 1 + szBytes + len
//            println(s"Will write $outLen bytes; len $len szBytes $szBytes")
            val debug = ByteVector(withOutput(outLen, write)).toHex
            println(s"Written $debug")

            write(out)
        }
      }
    )
  }
}
