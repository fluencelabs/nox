package fluence.node.config.storage

import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}
import io.circe.{Decoder, Encoder}

/**
 * Configuration for Ipfs storage, part of [[RemoteStorageConfig]]
 *
 * @param address URI of an Ipfs node
 */
case class IpfsConfig(address: String)

object IpfsConfig {
  implicit val encoder: Encoder[IpfsConfig] = deriveEncoder
  implicit val decoder: Decoder[IpfsConfig] = deriveDecoder
}
