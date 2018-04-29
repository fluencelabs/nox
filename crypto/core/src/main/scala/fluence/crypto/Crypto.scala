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

package fluence.crypto

import fluence.codec.{CodecError, MonadicalEitherArrow, PureCodec}

object Crypto extends MonadicalEitherArrow[CryptoError] {
  type Hasher[A, B] = Func[A, B]

  type Cipher[A] = Bijection[A, Array[Byte]]

  type KeyPairGenerator = Func[Option[Array[Byte]], KeyPair]

  // TODO: move it to MonadicalEitherArrow, make liftTry with try-catch, and easy conversions for other funcs and bijections
  implicit val liftCodecErrorToCrypto: CodecError ⇒ CryptoError = err ⇒ CryptoError("Codec error", Some(err))

  implicit def codec[A, B](implicit codec: PureCodec[A, B]): Bijection[A, B] =
    Bijection(fromOtherFunc(codec.direct), fromOtherFunc(codec.inverse))
}
