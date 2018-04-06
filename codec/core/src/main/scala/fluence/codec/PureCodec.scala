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

package fluence.codec

import scodec.bits.{Bases, ByteVector}

/**
 * PureCodec default values.
 */
object PureCodec extends MonadicalEitherArrow[CodecError] {

  implicit val byteArrayToVector: PureCodec[Array[Byte], ByteVector] =
    liftB(ByteVector.apply, _.toArray)

  implicit val base64ToVector: PureCodec[String, ByteVector] =
    liftEitherB(
      str ⇒
        ByteVector
          .fromBase64Descriptive(str, Bases.Alphabets.Base64)
          .left
          .map(CodecError(_)),
      vec ⇒ Right(vec.toBase64(Bases.Alphabets.Base64))
    )

  /**
   * Summons an implicit codec.
   */
  def codec[A, B](implicit pc: PureCodec[A, B]): PureCodec[A, B] = pc

  /**
   * Shortcut to build a PureCodec.Bijection with two Func's
   */
  def apply[A, B](direct: Func[A, B], inverse: Func[B, A]): Bijection[A, B] =
    Bijection(direct, inverse)

  /**
   * Shortcut to build a PureCodec.Bijection with two pure functions
   */
  def apply[A, B](direct: A ⇒ B, inverse: B ⇒ A): Bijection[A, B] =
    liftB(direct, inverse)
}
