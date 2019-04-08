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

package fluence.effects.ipfs

import io.circe.{Decoder, Encoder}
import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}

/**
 * File in IPFS.
 */
case class FileManifest(Name: String, Hash: String, Size: Int, Type: Int)

/**
 * IPFS object. Represents hash of object with links to contained files.
 *
 */
case class IpfsObject(Hash: String, Links: List[FileManifest])

/**
 * `ls` response from IPFS
 */
case class IpfsLsResponse(Objects: List[IpfsObject])

object IpfsLsResponse {
  implicit val encodeFileManifest: Encoder[FileManifest] = deriveEncoder
  implicit val decodeFileManifest: Decoder[FileManifest] = deriveDecoder
  implicit val encodeIpfsObject: Encoder[IpfsObject] = deriveEncoder
  implicit val decodeIpfsObject: Decoder[IpfsObject] = deriveDecoder
  implicit val encodeIpfsLs: Encoder[IpfsLsResponse] = deriveEncoder
  implicit val decodeIpfsLs: Decoder[IpfsLsResponse] = deriveDecoder
}
