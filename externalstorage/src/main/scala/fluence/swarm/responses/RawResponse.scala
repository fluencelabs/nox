/*
 * Copyright (C) 2018  Fluence Labs Limited
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

package fluence.swarm.responses
import io.circe.{Decoder, Encoder}
import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}

/**
 * One file in the manifest with additional information.
 *
 * @param hash address of file in swarm
 * @param contentType type of stored data (tar, multipart, etc)
 * @param mod_time time of modification
 */
case class Entrie(hash: String, contentType: String, mod_time: String)

object Entrie {
  implicit val fooDecoder: Decoder[Entrie] = deriveDecoder
  implicit val fooEncoder: Encoder[Entrie] = deriveEncoder
}

/**
 * Representation of raw manifest with entries.
 *
 * @param entries list of files under manifest
 */
case class RawResponse(entries: List[Entrie])

case object RawResponse {
  implicit val fooDecoder: Decoder[RawResponse] = deriveDecoder
  implicit val fooEncoder: Encoder[RawResponse] = deriveEncoder
}
