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

  def encode(bv: ByteVector): Array[Byte] = {
    if (bv.nonEmpty) {
      val bs = ByteString.copyFrom(bv.toArray)
      withOutput(bytesSize(bs), _.writeBytesNoTag(bs))
    } else {
      Array.empty
    }
  }
  def encode[T <: GeneratedMessage](m: Option[T]): Array[Byte] = m.fold(Array.empty[Byte])(encode(_))
  def encode[T <: GeneratedMessage](m: T): Array[Byte] = m.toByteArray
}
