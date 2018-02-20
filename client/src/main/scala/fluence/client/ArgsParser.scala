package fluence.client

import java.io.File

import fluence.crypto.KeyStore
import scopt.Read.reads
import scopt.{ OptionParser, Read }

case class CommandLineConfig(config: Option[File] = None, seed: Seq[String] = Seq.empty, keyStore: Option[KeyStore] = None)

object ArgsParser {
  implicit val keyStoreRead: Read[KeyStore] = {
    reads { str ⇒
      KeyStore.fromBase64(str)
    }
  }

  val parser = new OptionParser[CommandLineConfig]("scopt") {
    head("Fluence client")

    opt[File]('c', "config").valueName("<file>")
      .action((x, c) ⇒ c.copy(config = Some(x)))
      .text("Path to config file")

    opt[Seq[String]]('s', "seed").valueName("<seed1>,<seed2>...")
      .action((x, c) ⇒ c.copy(seed = x))
      .text("Initial kademlia nodes contacts in base64 to connect with")

    opt[KeyStore]('k', "keystore").valueName("<keystore>")
      .action((x, c) ⇒ c.copy(keyStore = Some(x)))
      .text("Key pair in base64")

    help("help").text("Write help message")

    note("Arguments will override values in config file")
  }
}
