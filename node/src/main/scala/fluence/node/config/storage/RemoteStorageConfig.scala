package fluence.node.config.storage

import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}
import io.circe.{Decoder, Encoder}

/**
 * Configuration for remote decentralized content-addressable stores used by Node
 *
 * @param enabled Disables remote storage, [[fluence.node.code.LocalCodeStore]] is used in that case
 * @param swarm Config for Swarm
 * @param ipfs Config for Ipfs
 */
case class RemoteStorageConfig(enabled: Boolean, swarm: SwarmConfig, ipfs: IpfsConfig)

object RemoteStorageConfig {
  implicit val encoder: Encoder[RemoteStorageConfig] = deriveEncoder
  implicit val decoder: Decoder[RemoteStorageConfig] = deriveDecoder
}
