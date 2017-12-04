package fluence.network

import com.typesafe.config.{ Config, ConfigFactory }

import scala.concurrent.duration.Duration

/**
 *
 * @param maxSiblingsSize Maximum number of siblings to store, e.g. K * Alpha
 * @param maxBucketSize Maximum size of a bucket, usually K
 * @param parallelism Parallelism factor (named Alpha in paper)
 * @param pingExpiresIn Duration to avoid too frequent ping requests, used in [[fluence.kad.Bucket.update()]]
 */
case class KademliaConf(
    maxBucketSize: Int,
    maxSiblingsSize: Int,
    parallelism: Int,
    pingExpiresIn: Duration
)

object KademliaConf {
  val ConfigPath = "fluence.network.kademlia"

  def read(path: String = ConfigPath, config: Config = ConfigFactory.load()): KademliaConf = {
    import net.ceedubs.ficus.Ficus._
    import net.ceedubs.ficus.readers.ArbitraryTypeReader._
    config.as[KademliaConf](path)
  }
}

