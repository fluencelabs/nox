package fluence.btree.server

import com.typesafe.config.{ Config, ConfigFactory }

/**
 * Configuration for [[MerkleBTree]]
 *
 * @param arity Maximum size of node (max number of tree node keys)
 * @param alpha Minimum capacity factor of node. Should be between 0 and 0.5.
 *               0.25 means that each node except root should always contains between 25% and 100% children.
 */
case class MerkleBTreeConfig(
    arity: Int = 8,
    alpha: Double = 0.25D
) {
  require(arity % 2 == 0, "arity should be even")
  require(0 < alpha && alpha < 0.5, "alpha should be between 0 and 0.5")
}

object MerkleBTreeConfig {

  val ConfigPath = "fluence.merkle.btree"

  def read(configPath: String = ConfigPath, conf: Config = ConfigFactory.load()): MerkleBTreeConfig = {
    import net.ceedubs.ficus.Ficus._
    import net.ceedubs.ficus.readers.ArbitraryTypeReader._
    conf.as[MerkleBTreeConfig](configPath)
  }

}
