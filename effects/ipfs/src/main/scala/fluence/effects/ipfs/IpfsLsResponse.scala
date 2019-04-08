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
