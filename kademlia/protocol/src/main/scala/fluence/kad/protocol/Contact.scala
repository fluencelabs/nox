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

package fluence.kad.protocol

import java.net.InetAddress

import cats.{ Eval, Show }
import scodec.bits.{ Bases, BitVector, ByteVector }
import scodec.Codec

case class Contact(
    ip: InetAddress,
    port: Int,

    protocolVersion: Long,
    gitHash: String
) {

  // TODO: make it pure
  lazy val binary: ByteVector =
    Contact.sCodec.encode(this).toEither match {
      case Right(s)  ⇒ s.toByteVector
      case Left(err) ⇒ throw new RuntimeException(err.toString())
    }

  lazy val b64seed: String =
    binary.toBase64(Bases.Alphabets.Base64)

  lazy val isLocal: Boolean =
    ip.isLoopbackAddress ||
      ip.isAnyLocalAddress ||
      ip.isLinkLocalAddress ||
      ip.isSiteLocalAddress ||
      ip.isMCSiteLocal ||
      ip.isMCGlobal ||
      ip.isMCLinkLocal ||
      ip.isMCNodeLocal ||
      ip.isMCOrgLocal
}

object Contact {
  implicit val show: Show[Contact] =
    (c: Contact) ⇒ s"${c.ip.getHostAddress}:${c.port}(${c.b64seed})[protoc:${c.protocolVersion} build:${c.gitHash}]"

  implicit val sCodec: Codec[Contact] = {
    import scodec.codecs._

    val gitHash = bytes(20).xmap[String](
      _.toHex(Bases.Alphabets.HexLowercase),
      ByteVector.fromHex(_, Bases.Alphabets.HexLowercase).getOrElse(ByteVector.fill(20)(0))
    )

    val inetAddress = bytes(4).xmap[InetAddress](
      bv ⇒ InetAddress.getByAddress(bv.toArray),
      ia ⇒ ByteVector(ia.getAddress)
    )

    (int64 ~ inetAddress ~ int32 ~ gitHash).xmap[Contact](
      {
        case (((v, ia), p), gh) ⇒
          Contact(ip = ia, port = p, protocolVersion = v, gitHash = gh)
      },
      c ⇒ (((c.protocolVersion, c.ip), c.port), c.gitHash)
    )
  }

  // TODO: make it pure
  def readBinary(bv: ByteVector): Eval[Contact] = Eval.later(
    sCodec.decodeValue(bv.toBitVector).toEither match {
      case Right(v)  ⇒ v
      case Left(err) ⇒ throw new RuntimeException(err.toString())
    }
  )

  // TODO: make it pure
  def readB64seed(str: String): Eval[Contact] = Eval.later{
    ByteVector
      .fromBase64(str, Bases.Alphabets.Base64).getOrElse(throw new RuntimeException("Can't decode seed"))
  }.flatMap(readBinary)
}
