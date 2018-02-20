package fluence.client

import cats.ApplicativeError
import com.typesafe.config.Config
import fluence.kad.KademliaConf

import scala.language.higherKinds

object KademliaConfigParser {
  def readKademliaConfig[F[_]](config: Config, path: String = "fluence.network.kademlia")(implicit F: ApplicativeError[F, Throwable]): F[KademliaConf] =
    F.catchNonFatal {
      import net.ceedubs.ficus.Ficus._
      import net.ceedubs.ficus.readers.ArbitraryTypeReader._
      config.as[KademliaConf](path)
    }
}
