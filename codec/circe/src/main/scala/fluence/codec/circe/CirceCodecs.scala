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

package fluence.codec.circe

import fluence.codec.{CodecError, PureCodec}
import io.circe._

object CirceCodecs {

  implicit def circeJsonCodec[T](encoder: Encoder[T], decoder: Decoder[T]): PureCodec[T, Json] =
    PureCodec.liftEitherB[T, Json](
      t ⇒ Right(encoder.apply(t)),
      json ⇒ decoder.decodeJson(json).left.map(f ⇒ CodecError("Cannot decode json value", causedBy = Some(f)))
    )

  implicit val circeJsonParseCodec: PureCodec[Json, String] =
    PureCodec.liftEitherB(
      json ⇒ Right(json.noSpaces),
      str ⇒ parser.parse(str).left.map(f ⇒ CodecError("Cannot parse json string", causedBy = Some(f)))
    )

}
