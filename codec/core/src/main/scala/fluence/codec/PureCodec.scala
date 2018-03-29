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

import scala.language.higherKinds

/**
 * PureCodec builder functions
 */
object PureCodec {

  def apply[A, B](f: FuncE[CodecError, A, B], g: FuncE[CodecError, B, A]): PureCodec[A, B] =
    BifuncE(f, g)

  def lift[A, B](f: A ⇒ B, g: B ⇒ A): PureCodec[A, B] =
    BifuncE.lift(f, g)

  def liftEither[A, B](f: A ⇒ Either[CodecError, B], g: B ⇒ Either[CodecError, A]): PureCodec[A, B] =
    BifuncE.liftEither(f, g)

}
