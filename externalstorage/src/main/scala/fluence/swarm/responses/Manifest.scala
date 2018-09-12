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

package fluence.swarm.responses
import io.circe.generic.extras.{ConfiguredJsonCodec, JsonKey}
import io.circe.{Decoder, Encoder, HCursor, Json}
import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}

/**
 * One file in the manifest with additional information.
 *
 * @param hash address of file in swarm
 * @param contentType type of stored data (tar, multipart, etc)
 * @param modTime time of modification
 */
case class Entry(hash: String, contentType: String, modTime: String)

object Entry {

  implicit val entryDecoder: Encoder[Entry] = new Encoder[Entry] {
    final def apply(e: Entry): Json = Json.obj(
      ("hash", Json.fromString(e.hash)),
      ("contentType", Json.fromString(e.contentType)),
      ("mod_time", Json.fromString(e.modTime))
    )
  }
  implicit val entryEncoder: Decoder[Entry] = new Decoder[Entry] {
    final def apply(c: HCursor): Decoder.Result[Entry] =
      for {
        hash <- c.downField("hash").as[String]
        contentType <- c.downField("contentType").as[String]
        modTime <- c.downField("mod_time").as[String]
      } yield {
        new Entry(hash, contentType, modTime)
      }
  }
}

/**
 * Representation of raw manifest with entries.
 *
 * @param entries list of files under manifest
 */
case class Manifest(entries: List[Entry])

case object Manifest {
  implicit val rawDecoder: Decoder[Manifest] = deriveDecoder
  implicit val rawEncoder: Encoder[Manifest] = deriveEncoder
}
