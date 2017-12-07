package fluence.network.client

import com.typesafe.config.{ Config, ConfigFactory }

case class ClientConf(keyHeader: String, contactHeader: String)

object ClientConf {
  val ConfigPath = "fluence.network.client"

  def read(path: String = ConfigPath, config: Config = ConfigFactory.load()): ClientConf = {
    import net.ceedubs.ficus.Ficus._
    import net.ceedubs.ficus.readers.ArbitraryTypeReader._
    config.as[ClientConf](path)
  }
}
