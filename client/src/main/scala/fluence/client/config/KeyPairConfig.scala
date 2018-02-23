package fluence.client.config

import cats.effect.IO
import com.typesafe.config.Config

case class KeyPairConfig(
    keyPath: String
)

object KeyPairConfig {
  def read(config: Config): IO[KeyPairConfig] =
    IO {
      import net.ceedubs.ficus.Ficus._
      import net.ceedubs.ficus.readers.ArbitraryTypeReader._
      config.as[KeyPairConfig]("fluence.keys")
    }
}
