package fluence.network.server

import com.typesafe.config.{ Config, ConfigFactory }

case class ServerConf(
    localPort: Int,
    externalPort: Option[Int]
)

object ServerConf {
  val ConfigPath = "fluence.network.server"

  def read(path: String = ConfigPath, config: Config = ConfigFactory.load()): ServerConf = {
    import net.ceedubs.ficus.Ficus._
    import net.ceedubs.ficus.readers.ArbitraryTypeReader._
    config.as[ServerConf](path)
  }
}
